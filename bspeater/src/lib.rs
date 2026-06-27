#![allow(clippy::type_complexity)]
#![feature(seek_stream_len, iter_array_chunks)]

use avian3d::prelude::*;
#[cfg(not(feature = "graphics"))]
use bevy::mesh::MeshPlugin;
use bevy::{
    mesh::{MeshVertexAttribute, VertexFormat},
    prelude::*,
    state::app::StatesPlugin,
};
use oktree::{prelude::*, tree::Octree};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use std::{
    io::{self, Read, Seek, SeekFrom},
    ops::Div,
};

use crate::bindings::{BSPHeader, LumpHeader, LumpIds};

pub mod bindings;
pub mod export;
pub mod geoset_loader;
pub mod mdl_loader;
pub mod saving;

pub const CELL_SIZE: f32 = 25.;

pub const ATTRIBUTE_PRIMATIVE_TYPE: MeshVertexAttribute =
    MeshVertexAttribute::new("Primative_Type", 2001, VertexFormat::Uint32);
pub const ATTRIBUTE_UNIQUE_CONTENTS: MeshVertexAttribute =
    MeshVertexAttribute::new("Unique_Contents", 2000, VertexFormat::Sint32);
pub const OFFSET: i32 = i32::MAX / 2;

pub trait SeekRead: Seek + Read {}
impl<T: Seek + Read> SeekRead for T {}

pub trait VPKReader {
    fn read_vpk_file(&self, path: &Path) -> Result<Vec<u8>, std::io::Error>;
}

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct WorldExtends(Vec3, Vec3);

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct EnabledFeatures {
    pub grid: bool,
    pub octree: bool,
    pub no_export_obj: bool,
}

#[derive(Resource, Clone, PartialEq)]
pub struct WorldName {
    pub map_name: String,
    pub output: PathBuf,
}

#[cfg_attr(not(feature = "graphics"), allow(unused))]
#[derive(Resource, Default, Debug, Clone)]
pub struct ChunkCells {
    pub tree: Octree<u32, TUVec3u32>,
    pub collied_vec: Vec<[u32; 3]>,
}

#[derive(Debug, States, Hash, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ProcessingStep {
    #[default]
    Startup,
    RayCasting,
    Saving,
    Done,
    Exit,
}

pub fn read_i32(reader: &mut dyn SeekRead) -> Result<i32, io::Error> {
    let mut int = [0; size_of::<i32>()];
    reader.read_exact(&mut int)?;
    Ok(i32::from_le_bytes(int))
}

pub fn read_lump(reader: &mut dyn SeekRead) -> Result<LumpHeader, io::Error> {
    Ok(LumpHeader {
        fileofs: read_i32(reader)?,
        filelen: read_i32(reader)?,
        version: read_i32(reader)?,
        four_cc: read_i32(reader)?,
    })
}

pub fn read_bspheader(reader: &mut dyn SeekRead) -> Result<BSPHeader, io::Error> {
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

pub fn read_lump_data<T>(
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

pub fn get_lump(header: &BSPHeader, lump: LumpIds) -> &LumpHeader {
    &header.lumps[lump as usize]
}

pub fn generate_meshes_from_bsp<T, R>(
    mut bsp: T,
    vpk_reader: R,
    map_name: &str,
) -> Result<Vec<Mesh>, anyhow::Error>
where
    T: SeekRead,
    R: VPKReader,
{
    use crate::bindings::*;

    let header = read_bspheader(&mut bsp)?;
    let vertices = read_lump_data::<Vec3>(&mut bsp, &header, LumpIds::VERTICES)?;
    let normals = read_lump_data::<Vec3>(&mut bsp, &header, LumpIds::VERTEX_NORMALS)?;
    let tricoll_headers =
        read_lump_data::<TricollHeader>(&mut bsp, &header, LumpIds::TRICOLL_HEADERS)?;
    let tricoll_triangles =
        read_lump_data::<TricollTri>(&mut bsp, &header, LumpIds::TRICOLL_TRIANGLES)?;
    let geo_sets = read_lump_data::<GeoSet>(&mut bsp, &header, LumpIds::CM_GEO_SETS)?;
    let col_primitives =
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
    let (props, model_data) = mdl_loader::extract_game_lump_models(game_lump, vpk_reader);
    println!("vertices {:#?}", vertices.len());
    println!("normals {:#?}", normals.len());
    let meshes = geoset_loader::geoset_to_meshes(
        BSPData {
            vertices,
            tricoll_headers,
            tricoll_triangles,
            geo_sets,
            col_primitives,
            unique_contents,
            brushes,
            brush_side_plane_offsets,
            brush_planes,
            grid,
            props,
            model_data,
        },
        map_name,
    );
    Ok(meshes)
}

pub fn create_navmesh<T, R>(
    map_name: &str,
    bsp: T,
    vpk_reader: R,
    output_path: PathBuf,
) -> anyhow::Result<()>
where
    T: SeekRead,
    R: VPKReader,
{
    let mut app = App::new();
    app.add_plugins((
        // no graphics
        MinimalPlugins,
        AssetPlugin::default(),
        MeshPlugin,
        StatesPlugin,
        PhysicsPlugins::default(),
    ))
    .init_resource::<ChunkCells>()
    .insert_resource(WorldName {
        map_name: map_name.to_owned(),
        output: output_path,
    })
    .insert_resource(EnabledFeatures {
        grid: false,
        octree: false,
        no_export_obj: true,
    })
    .init_state::<ProcessingStep>();

    add_meshes_to_world(
        generate_meshes_from_bsp(bsp, vpk_reader, map_name)?,
        &mut app,
    );

    app.add_plugins(navmesh_generation_plugin)
        .add_systems(
            Update,
            exit_app_system.run_if(in_state(ProcessingStep::Exit)),
        )
        .run();

    Ok(())
}

pub fn add_meshes_to_world(meshes: Vec<Mesh>, app: &mut App) {
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
            #[cfg(not(feature = "graphics"))]
            let _ = i;
            Some((
                Collider::trimesh_from_mesh(&mesh)?,
                RigidBody::Static,
                Mesh3d(
                    app.world_mut()
                        .get_resource_mut::<Assets<Mesh>>()
                        .expect("this should exist probably")
                        .add(mesh),
                ),
                #[cfg(feature = "graphics")]
                MeshMaterial3d(materials[i % 3].clone()),
            ))
        })
        .collect::<Vec<_>>()
    {
        app.world_mut().spawn(mesh);
    }
}

pub fn navmesh_generation_plugin(app: &mut App) {
    app.add_systems(Startup, calc_extents).add_systems(
        Update,
        (
            raycast_world.run_if(in_state(ProcessingStep::RayCasting)),
            save_navmesh.run_if(in_state(ProcessingStep::Saving)),
            export::save_meshes.run_if(in_state(ProcessingStep::Done)),
        ),
    );
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
    commands.insert_resource(WorldExtends(min * reduce, max * reduce));
    next_state.set(ProcessingStep::RayCasting);
}

fn raycast_world(
    mut commands: Commands,
    ray_cast: SpatialQuery,
    extends: Res<WorldExtends>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    let extends = *extends;
    let cuboid = Collider::cuboid(CELL_SIZE, CELL_SIZE, CELL_SIZE);
    let mut scale_cuboid = cuboid.clone();
    scale_cuboid.scale_by(extends.0.abs() + extends.1.abs(), 1);

    // cast a shape cast over the whole world because it takes a few frames for avian get collisions up and running
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

    let octtree = Octree::<u32, TUVec3u32>::from_aabb(Aabb::from_min_max(
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
            )
        })
        .filter(|(_, hit, _)| *hit)
        .map(move |([x, y, z], _, _near_wall)| {
            [x + OFFSET, y + OFFSET, z + OFFSET].map(|v| v as u32)
        })
        .collect::<Vec<[u32; 3]>>();

    #[cfg(feature = "graphics")]
    for cell in full_vec.iter().cloned() {
        // look into this
        _ = octtree.insert(TUVec3u32::new(cell[0], cell[1], cell[2]));
    }

    bevy::log::info!("navmesh points: {}", full_vec.len());

    commands.remove_resource::<ChunkCells>();
    commands.insert_resource(ChunkCells {
        tree: octtree,
        collied_vec: full_vec,
    });
    next_state.set(ProcessingStep::Saving);
}

fn save_navmesh(
    map_name: Res<WorldName>,
    extends: Res<WorldExtends>,
    cells: Res<ChunkCells>,
    mut next_state: ResMut<NextState<ProcessingStep>>,
) {
    saving::save_navmesh_to_disk(
        cells
            .collied_vec
            .iter()
            .cloned()
            .map(|inter| UVec3::from_array(inter).as_ivec3() - IVec3::splat(OFFSET))
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

pub fn exit_app_system(mut writer: MessageWriter<AppExit>) {
    writer.write(AppExit::Success);
}

pub fn map_to_u32(value: i32) -> u32 {
    (value + OFFSET) as u32
}

pub fn map_to_i32(value: u32) -> i32 {
    value as i32 - OFFSET
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
