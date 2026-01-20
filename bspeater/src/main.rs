#![allow(clippy::type_complexity)]
#![feature(seek_stream_len, iter_array_chunks)]

use anyhow::Context;
use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    mesh::{MeshVertexAttribute, VertexFormat},
    pbr::wireframe::WireframeConfig,
    prelude::*,
    render::{RenderPlugin, settings::WgpuSettings},
};
use clap::Parser;
use itertools::Itertools;
use oktree::{prelude::*, tree::Octree};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    ops::{Div, Not},
    path::{Path, PathBuf},
    process::Command,
};

pub use bindings::*;

mod async_pathfinding;
mod behavior;
mod bindings;
mod cli;
mod debug;
mod export;
mod geoset_loader;
mod mdl_loader;
mod pathfinding;
mod saving;

pub const UNPACK: &str = "vpk";
pub const UNPACK_MERGED: &str = "vpk_merged";
pub const UNPACK_COMMON: &str = "common_vpk";

pub const ATTRIBUTE_PRIMATIVE_TYPE: MeshVertexAttribute =
    MeshVertexAttribute::new("Primative_Type", 2001, VertexFormat::Uint32);
pub const ATTRIBUTE_UNIQUE_CONTENTS: MeshVertexAttribute =
    MeshVertexAttribute::new("Unique_Contents", 2000, VertexFormat::Sint32);

trait SeekRead: Seek + Read {}
impl<T: Seek + Read> SeekRead for T {}

#[derive(Component)]
struct WorldMesh;

fn read_i32(reader: &mut dyn SeekRead) -> Result<i32, io::Error> {
    let mut int = [0; size_of::<i32>()];
    reader.read_exact(&mut int)?;
    Ok(i32::from_le_bytes(int))
}

fn read_lump(reader: &mut dyn SeekRead) -> Result<LumpHeader, io::Error> {
    Ok(LumpHeader {
        fileofs: read_i32(reader)?,
        filelen: read_i32(reader)?,
        version: read_i32(reader)?,
        four_cc: read_i32(reader)?,
    })
}

fn read_bspheader(reader: &mut dyn SeekRead) -> Result<BSPHeader, io::Error> {
    reader.seek(SeekFrom::Start(0))?;

    let mut magic = [0; 4];
    reader.read_exact(&mut magic)?;
    let version = read_i32(reader)?;

    assert_eq!(&magic, b"rBSP");
    assert_eq!(version, 37);

    Ok(BSPHeader {
        filemagic: magic,
        version,
        map_revisions: read_i32(reader)?,
        _127: read_i32(reader)?,
        lumps: (0..128)
            .map(|_| read_lump(reader))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| io::Error::other("incorrect size for lumps how!"))?,
    })
}

fn read_lump_data<T>(
    reader: &mut dyn SeekRead,
    header: &BSPHeader,
    id: LumpIds,
) -> Result<Vec<T>, io::Error> {
    let lump = get_lump(header, id);
    let size = std::mem::size_of::<T>();

    reader.seek(SeekFrom::Start(lump.fileofs as u64))?;

    let mut buf = vec![0; lump.filelen as usize];

    reader.read_exact(&mut buf)?;

    assert!(buf.len().is_multiple_of(size), "lump {id:?}");
    assert!(buf.capacity().is_multiple_of(size), "lump {id:?}");

    let tricoll = unsafe {
        Vec::<T>::from_raw_parts(
            buf.as_ptr().cast_mut().cast(),
            buf.len() / size,
            buf.capacity() / size,
        )
    };

    std::mem::forget(buf);

    Ok(tricoll)
}

fn get_lump(header: &BSPHeader, lump: LumpIds) -> &LumpHeader {
    &header.lumps[lump as usize]
}

fn main() -> anyhow::Result<()> {
    let cli::BspeaterCli {
        vpk_dir,
        game_dir,
        display,
        map_name,
        show_octtree,
        show_grid_octtree,
        output,
    } = cli::BspeaterCli::parse();

    let name = format!("englishclient_{map_name}.bsp.pak000_dir.vpk");
    let vpk_name_magic = vpk_dir
        .join(UNPACK)
        .join("current_vpk")
        .display()
        .to_string();

    // put a file to indicate what vpk is open then clean the vpk dir if we are opening another vpk
    std::fs::create_dir_all(vpk_dir.join(UNPACK_MERGED))
        .context("tried creating merged unpack dir")?;
    {
        std::fs::create_dir_all(vpk_dir.join(UNPACK)).context("tried creating unpack dir wow")?;
        _ = File::create_new(&vpk_name_magic);

        if std::fs::read_to_string(&vpk_name_magic).context("tried reading current vpk name")?
            != map_name
        {
            std::fs::remove_dir_all(vpk_dir.join(UNPACK)).context("tried removing unpack dir")?;
        }
    }

    if !vpk_dir.join(UNPACK_COMMON).is_dir() {
        let lumps = (0..128).flat_map(|i| ["--exclude-bsp-lump".to_string(), i.to_string()]);
        Command::new("tf2-vpkunpack")
            .args(lumps)
            .arg("--exclude")
            .arg("*")
            .arg("--include")
            .arg("models/")
            .arg("--include")
            .arg("maps/")
            .arg(vpk_dir.join(UNPACK_COMMON))
            .arg(game_dir.join("englishclient_mp_common.bsp.pak000_dir.vpk"))
            .spawn()
            .context("tried spawning the unpacking command")?
            .wait_with_output()
            .context("tried unpacking common vpk")?;

        std::fs::create_dir_all(vpk_dir.join(UNPACK_MERGED))
            .context("tried creating merged dir")?;
        copy_dir_all(vpk_dir.join(UNPACK_COMMON), vpk_dir.join(UNPACK_MERGED))
            .context("tried merging common vpk")?;
    }

    let mut bsp =
        if !vpk_dir.join(&map_name).with_extension("bsp").exists() && map_name != "mp_lobby" {
            Command::new("tf2-vpkunpack")
                .arg("--exclude")
                .arg("*")
                .arg("--include")
                .arg("maps")
                .arg("--include")
                .arg("models")
                .arg(vpk_dir.join(UNPACK))
                .arg(game_dir.join(name))
                .spawn()?
                .wait_with_output()
                .context("tried unpacking vpks")?;

            copy_dir_all(vpk_dir.join(UNPACK), vpk_dir.join(UNPACK_MERGED))
                .context("tried merging vpks")?;

            File::open(
                vpk_dir
                    .join(UNPACK_MERGED)
                    .join("maps")
                    .join(&map_name)
                    .with_extension("bsp"),
            )
            .context("tried getting unpacked map")?
        } else if map_name == "mp_lobby" {
            std::fs::create_dir_all(vpk_dir.join(UNPACK)).context("tried creating unpack dir")?;
            File::open(
                vpk_dir
                    .join(UNPACK_MERGED)
                    .join("maps")
                    .join("mp_lobby")
                    .with_extension("bsp"),
            )
            .context("tried getting mp_lobby")?
        } else {
            std::fs::create_dir_all(vpk_dir.join(UNPACK)).context("tried creating unpack dir")?;
            File::open(vpk_dir.join(&map_name).with_extension("bsp"))
                .context("tried getting custom bsp")?
        };

    {
        let mut current_vpk =
            File::create(&vpk_name_magic).context("tried creating current vpk")?;
        _ = current_vpk
            .write(map_name.as_bytes())
            .context("tried setting current vpk")?;
    }

    assert!(std::mem::size_of::<Vec3>() == std::mem::size_of::<f32>() * 3);

    let header = read_bspheader(&mut bsp)?;
    let vertices = read_lump_data::<Vec3>(&mut bsp, &header, LumpIds::VERTICES)?;
    let normals = read_lump_data::<Vec3>(&mut bsp, &header, LumpIds::VERTEX_NORMALS)?;
    // let mesh_indices = read_lump_data::<u16>(&mut bsp, &header, LumpIds::MESH_INDICES)?;
    // let bspmeshes = read_lump_data::<BspMesh>(&mut bsp, &header, LumpIds::MESHES)?;
    // let materialsorts = read_lump_data::<MaterialSort>(&mut bsp, &header, LumpIds::MATERIAL_SORTS)?;
    // let vertex_unlit = read_lump_data::<VertexUnlit>(&mut bsp, &header, LumpIds::VERTEX_UNLIT)?;
    // let vertex_lit_flat =
    //     read_lump_data::<VertexLitFlat>(&mut bsp, &header, LumpIds::VERTEX_LIT_FLAT)?;
    // let vertex_lit_bump =
    //     read_lump_data::<VertexLitBump>(&mut bsp, &header, LumpIds::VERTEX_LIT_BUMP)?;
    // let vertex_unlit_ts =
    //     read_lump_data::<VertexUnlitTS>(&mut bsp, &header, LumpIds::VERTEX_UNLIT_TS)?;

    let tricoll_headers =
        read_lump_data::<TricollHeader>(&mut bsp, &header, LumpIds::TRICOLL_HEADERS)?;
    let tricoll_triangles =
        read_lump_data::<TricollTri>(&mut bsp, &header, LumpIds::TRICOLL_TRIANGLES)?;
    // let texture_data = read_lump_data::<Dtexdata>(&mut bsp, &header, LumpIds::TEXTURE_DATA)?;
    let geo_sets = read_lump_data::<GeoSet>(&mut bsp, &header, LumpIds::CM_GEO_SETS)?;
    let col_primatives =
        read_lump_data::<CollPrimitive>(&mut bsp, &header, LumpIds::CM_PRIMITIVES)?;
    let unique_contents = read_lump_data::<i32>(&mut bsp, &header, LumpIds::CM_UNIQUE_CONTENTS)?;

    let brushes = read_lump_data::<Brush>(&mut bsp, &header, LumpIds::CM_BRUSHES)?;
    let brush_side_plane_offsets =
        read_lump_data::<u16>(&mut bsp, &header, LumpIds::CM_BRUSH_SIDE_PLANE_OFFSETS)?;
    let brush_planes = read_lump_data::<Vec4>(&mut bsp, &header, LumpIds::PLANES)?;
    let grid = read_lump_data::<CMGrid>(&mut bsp, &header, LumpIds::CM_GRID)?
        .first()
        .cloned()
        .ok_or_else(|| anyhow::format_err!("isn't there supposed to be only one grid thing"))?;

    let game_lump = read_lump_data::<u8>(&mut bsp, &header, LumpIds::GAME_LUMP)?;

    let (props, model_data) =
        mdl_loader::extract_game_lump_models(game_lump, vpk_dir.join(UNPACK_MERGED));

    println!("vertices {:#?}", vertices.len());
    println!("normals {:#?}", normals.len());

    let meshes = geoset_loader::geoset_to_meshes(
        BSPData {
            vertices,
            tricoll_headers,
            tricoll_triangles,
            geo_sets,
            col_primatives,
            unique_contents,
            brushes,
            brush_side_plane_offsets,
            brush_planes,
            grid,
            props,
            model_data,
        },
        &map_name,
    );

    let mut app = App::new();

    app.add_plugins((
        #[cfg(not(feature = "graphics"))]
        MinimalPlugins,
        #[cfg(feature = "graphics")]
        DefaultPlugins.set(RenderPlugin {
            render_creation: if display.not() {
                WgpuSettings {
                    backends: None,
                    ..default()
                }
                .into()
            } else {
                Default::default()
            },
            ..default()
        }),
        PhysicsPlugins::default(),
        #[cfg(feature = "graphics")]
        PhysicsDebugPlugin,
        // #[cfg(feature = "graphics")]
        // WireframePlugin::default(),
        #[cfg(feature = "graphics")]
        FreeCameraPlugin,
    ))
    .init_resource::<WireframeConfig>()
    .init_resource::<ChunkCells>()
    .add_systems(Startup, setup)
    .insert_resource(WorldName {
        map_name: map_name.to_owned(),
        output,
    })
    .insert_resource(EarlyExit(!display))
    .insert_resource(DebugAmount {
        grid: show_grid_octtree,
        octree: show_octtree,
    })
    .init_state::<ProcessingStep>();

    const BASE: u8 = 200;
    let materials = {
        let mut mat = app
            .world_mut()
            .get_resource_mut::<Assets<StandardMaterial>>()
            .expect("this should exist probably");
        [
            mat.add(StandardMaterial::from_color(Color::srgba_u8(
                BASE, 0, 0, 255,
            ))),
            mat.add(StandardMaterial::from_color(Color::srgba_u8(
                0, BASE, 0, 255,
            ))),
            mat.add(StandardMaterial::from_color(Color::srgba_u8(
                0, 0, BASE, 255,
            ))),
        ]
    };

    for mesh in meshes
        .into_iter()
        .filter(|mesh| {
            mesh.get_vertex_size() > 1
                && mesh
                    .indices()
                    .into_iter()
                    .flat_map(|indices| indices.iter())
                    .count()
                    > 1
        })
        .enumerate()
        .filter_map(|(i, mesh)| {
            Some((
                Collider::trimesh_from_mesh(&mesh)?,
                RigidBody::Static,
                Mesh3d(
                    app.world_mut()
                        .get_resource_mut::<Assets<Mesh>>()
                        .expect("this should exist probably")
                        .add(mesh),
                ),
                MeshMaterial3d(materials[i % 3].clone()),
                WorldMesh,
            ))
        })
        .collect::<Vec<_>>()
    {
        app.world_mut().spawn(mesh);
    }

    // not debugging needed when we don't even see an output
    if display {
        app.add_plugins(debug::debug_plugin);
    }

    app.add_systems(Startup, calc_extents)
        .add_systems(
            Update,
            (
                raycast_world.run_if(in_state(ProcessingStep::RayCasting)),
                save_navmesh.run_if(in_state(ProcessingStep::Saving)),
                export::save_meshes.run_if(in_state(ProcessingStep::Done)),
                exit_app_system
                    .run_if(in_state(ProcessingStep::Exit))
                    .run_if(|exit: Res<EarlyExit>| exit.0),
            ),
        )
        .run();

    Ok(())
}

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct WorlExtends(Vec3, Vec3);

#[derive(Resource, Clone, Copy, PartialEq)]
struct DebugAmount {
    grid: bool,
    octree: bool,
}

#[derive(Resource, Clone, PartialEq)]
struct WorldName {
    map_name: String,
    output: PathBuf,
}

#[derive(Resource, Clone, PartialEq)]
struct EarlyExit(bool);

#[derive(Component, Clone, Copy, PartialEq)]
struct WireMe;

#[derive(Debug, States, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
enum ProcessingStep {
    #[default]
    Startup,
    RayCasting,
    Saving,
    Done,
    Exit,
}

fn setup(mut commands: Commands, mut wireframe_config: ResMut<WireframeConfig>) {
    commands.spawn((
        Camera3d::default(),
        FreeCamera {
            walk_speed: 800.,
            run_speed: 400.,
            friction: 40.,
            sensitivity: 0.4,
            key_forward: KeyCode::KeyW,
            key_back: KeyCode::KeyS,
            key_left: KeyCode::KeyA,
            key_right: KeyCode::KeyD,
            key_up: KeyCode::KeyE,
            key_down: KeyCode::KeyQ,
            key_run: KeyCode::ShiftLeft,
            mouse_key_cursor_grab: MouseButton::Left,
            keyboard_key_toggle_cursor_grab: KeyCode::Space,
            ..default()
        },
    ));
    wireframe_config.global = true;
}

fn calc_extents(
    mut commands: Commands,
    meshes: Query<&Mesh3d>,
    assets: Res<Assets<Mesh>>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    let (min, max) = meshes
        .iter()
        .collect::<Vec<_>>()
        .into_par_iter()
        .filter_map(|mesh| assets.get(&mesh.0))
        .filter_map(|mesh| {
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .map(|pos| match pos {
                    bevy::mesh::VertexAttributeValues::Float32x3(vertexes) => vertexes
                        .iter()
                        .map(|pos| Vec3::from_array(*pos))
                        .fold((Vec3::ZERO, Vec3::ZERO), |current, cmp| {
                            (current.0.min(cmp), current.1.max(cmp))
                        }),
                    _ => panic!("vertex is not vertex"),
                })
        })
        .reduce(
            || (Vec3::ZERO, Vec3::ZERO),
            |current, cmp| (current.0.min(cmp.0), current.1.max(cmp.1)),
        );

    let reduce = Vec3::splat(1.);
    commands.insert_resource(WorlExtends(min * reduce, max * reduce));
    next_state.set(ProcessingStep::RayCasting);
}

const CELL_SIZE: f32 = 25.;
fn raycast_world(
    mut commands: Commands,
    ray_cast: SpatialQuery,
    extends: Res<WorlExtends>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    let shape_config: ShapeCastConfig = ShapeCastConfig {
        max_distance: 0.,
        compute_contact_on_penetration: false,
        ..default()
    };
    let extends = *extends;
    let cuboid = Collider::cuboid(CELL_SIZE, CELL_SIZE, CELL_SIZE);
    let mut scale_cuboid = cuboid.clone();
    scale_cuboid.scale_by(extends.0.abs() + extends.1.abs(), 1);

    // cast a shap cast over the whole world because it takes a few frames for avian get collisions up and running
    if ray_cast
        .shape_intersections(
            &scale_cuboid,
            Vec3::new(0., 0., 0.),
            Quat::default(),
            &SpatialQueryFilter::DEFAULT,
        )
        .is_empty()
    {
        bevy::log::info!("empty");
        return;
    }

    let (min, max) = (
        ((extends.0 / Vec3::splat(CELL_SIZE)).as_ivec3() + IVec3::splat(OFFSET))
            .as_uvec3()
            .to_array()
            .into_iter()
            .min()
            .expect("bruh how"),
        ((extends.1 / Vec3::splat(CELL_SIZE)).as_ivec3() + IVec3::splat(OFFSET))
            .as_uvec3()
            .to_array()
            .into_iter()
            .max()
            .expect("bruh how"),
    );

    let mut octtree = Octree::<u32, TUVec3u32>::from_aabb(Aabb::from_min_max(
        TUVec3 {
            x: round_down_to_power_of_2(min),
            y: round_down_to_power_of_2(min),
            z: round_down_to_power_of_2(min),
        },
        TUVec3 {
            x: round_up_to_power_of_2(max),
            y: round_up_to_power_of_2(max),
            z: round_up_to_power_of_2(max),
        },
    ));

    let full_vec = (extends.0.x.div(CELL_SIZE) as i32..=extends.1.x.div(CELL_SIZE) as i32)
        .into_par_iter()
        .flat_map_iter(move |x| {
            (extends.0.y.div(CELL_SIZE) as i32..=extends.1.y.div(CELL_SIZE) as i32).flat_map(
                move |y| {
                    (extends.0.z.div(CELL_SIZE) as i32..=extends.1.z.div(CELL_SIZE) as i32)
                        .map(move |z| IVec3::new(x, y, z))
                },
            )
        })
        .map(|vec| {
            let origin = vec.as_vec3() * Vec3::splat(CELL_SIZE);
            (
                vec.to_array(),
                !ray_cast
                    .shape_intersections(
                        &cuboid,
                        origin,
                        Quat::default(),
                        &SpatialQueryFilter::DEFAULT,
                    )
                    .is_empty(),
                true,
                0., // ray_cast
                    //     .cast_ray(
                    //         origin,
                    //         Dir3::X,
                    //         shape_config.target_distance,
                    //         false,
                    //         &SpatialQueryFilter::DEFAULT,
                    //     )
                    //     .is_some(),
                    // ray_cast
                    //     .cast_shape(
                    //         &cuboid,
                    //         origin,
                    //         Quat::default(),
                    //         Dir3::NEG_Y,
                    //         &shape_config,
                    //         &SpatialQueryFilter::DEFAULT,
                    //     )
                    //     .map(|hitdata| hitdata.distance)
                    //     .unwrap_or(CELL_SIZE),
            )
        })
        .filter(|(_, hit, _, _)| *hit)
        .map(move |([x, y, z], _, _near_wall, height)| ChunkCell {
            cord: [x + OFFSET, y + OFFSET, z + OFFSET].map(|v| v as u32),
            _floor_distance: height,
        })
        .collect::<Vec<ChunkCell>>();
    for cell in full_vec.iter().cloned() {
        // look into this
        if let Err(err) = octtree.insert(TUVec3u32::new(cell.cord[0], cell.cord[1], cell.cord[2])) {
            _ = err;
            // bevy::log::error!("tree: {err}");
        };
    }

    bevy::log::info!("navmesh points: {}", full_vec.len());

    commands.remove_resource::<ChunkCells>();
    commands.insert_resource(ChunkCells {
        tree: octtree,
        collied_vec: full_vec,
    });
    next_state.set(ProcessingStep::Saving);
}

#[derive(Debug, Default, Clone, Copy)]
struct ChunkCell {
    cord: [u32; 3],
    _floor_distance: f32,
}

#[derive(Resource, Default, Debug, Clone)]
struct ChunkCells {
    tree: Octree<u32, TUVec3u32>,
    collied_vec: Vec<ChunkCell>,
}

impl oktree::Position for ChunkCell {
    type U = u32;

    fn position(&self) -> oktree::prelude::TUVec3<Self::U> {
        TUVec3::new(self.cord[0], self.cord[1], self.cord[2])
    }
}

fn save_navmesh(
    map_name: Res<WorldName>,
    extends: Res<WorlExtends>,
    cells: Res<ChunkCells>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    saving::save_navmesh_to_disk(
        cells
            .collied_vec
            .iter()
            .map(|inter| UVec3::from_array(inter.cord).as_ivec3() - IVec3::splat(OFFSET))
            .collect(),
        (
            (extends.0 / Vec3::splat(CELL_SIZE)).as_ivec3(),
            (extends.1 / Vec3::splat(CELL_SIZE)).as_ivec3(),
        ),
        CELL_SIZE,
        &map_name.map_name,
        map_name.output.as_path(),
    );

    next_state.set(ProcessingStep::Done);
}

fn exit_app_system(mut writer: MessageWriter<AppExit>) {
    writer.write(AppExit::Success);
}

fn copy_dir_all(src: impl AsRef<Path>, dst: impl AsRef<Path>) -> io::Result<()> {
    use std::fs;
    fs::create_dir_all(&dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn round_up_to_power_of_2(mut num: u32) -> u32 {
    num = num.wrapping_sub(1);
    num |= num >> 1;
    num |= num >> 2;
    num |= num >> 4;
    num |= num >> 8;
    num |= num >> 16;
    num.wrapping_add(1)
}

fn round_down_to_power_of_2(num: u32) -> u32 {
    round_up_to_power_of_2(num) >> 1
}

// fn distance_to_origin(pos: &[i32; 3]) -> f32 {
//     (pos[0].pow(2) as f32 + pos[1].pow(2) as f32 + pos[2].pow(2) as f32).sqrt()
// }

const OFFSET: i32 = i32::MAX / 2;
pub fn map_to_u32(value: i32) -> u32 {
    (value + OFFSET) as u32
}

pub fn map_to_i32(value: u32) -> i32 {
    value as i32 - OFFSET
}
