#![allow(clippy::type_complexity)]
#![feature(seek_stream_len, iter_array_chunks)]

use anyhow::Context;
use avian3d::prelude::*;
#[cfg(not(feature = "graphics"))]
use bevy::mesh::MeshPlugin;
#[cfg(feature = "graphics")]
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::prelude::*;
#[cfg(feature = "graphics")]
use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    render::{RenderPlugin, settings::WgpuSettings},
};
use clap::Parser;
use lib::{ChunkCells, EnabledFeatures, ProcessingStep, WorldName, navmesh_generation_plugin};
#[cfg(feature = "graphics")]
use std::ops::Not;
use std::{
    fs::File,
    io::{self, Write},
    path::Path,
    process::Command,
};

mod cli;
#[cfg(feature = "graphics")]
mod debug;

pub const UNPACK: &str = "vpk";
pub const UNPACK_MERGED: &str = "vpk_merged";
pub const UNPACK_COMMON: &str = "common_vpk";

#[derive(Component)]
struct WorldMesh;

#[derive(Resource, Clone, PartialEq)]
pub struct EarlyExit(bool);

fn main() -> anyhow::Result<()> {
    let cli::BspeaterCli {
        vpk_dir,
        game_dir,
        display,
        map_name,
        show_octtree,
        show_grid_octtree,
        no_export_obj,
        output,
    } = cli::BspeaterCli::parse();
    let display = display && cfg!(feature = "graphics");

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

    let bsp = if !vpk_dir.join(&map_name).with_extension("bsp").exists() && map_name != "mp_lobby" {
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

    let meshes =
        lib::generate_meshes_from_bsp(bsp, vpk_dir.join(UNPACK_MERGED).to_path_buf(), &map_name)?;

    let mut app = App::new();

    app.add_plugins((
        // no graphics
        #[cfg(not(feature = "graphics"))]
        MinimalPlugins,
        #[cfg(not(feature = "graphics"))]
        AssetPlugin::default(),
        #[cfg(not(feature = "graphics"))]
        MeshPlugin,
        #[cfg(not(feature = "graphics"))]
        bevy::state::app::StatesPlugin,
        // standard
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
        // #[cfg(feature = "graphics")]
        // PhysicsDebugPlugin,
        #[cfg(feature = "graphics")]
        WireframePlugin::default(),
        #[cfg(feature = "graphics")]
        FreeCameraPlugin,
    ))
    .init_resource::<ChunkCells>()
    .add_systems(
        Startup,
        (
            setup_camera,
            #[cfg(feature = "graphics")]
            setup_wireframe,
        ),
    )
    .insert_resource(WorldName {
        map_name: map_name.to_owned(),
        output,
    })
    .insert_resource(EarlyExit(!display))
    .insert_resource(EnabledFeatures {
        grid: show_grid_octtree,
        octree: show_octtree,
        no_export_obj,
    })
    .init_state::<ProcessingStep>();

    #[cfg(feature = "graphics")]
    let materials = {
        const BASE: u8 = 200;
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
                WorldMesh,
            ))
        })
        .collect::<Vec<_>>()
    {
        app.world_mut().spawn(mesh);
    }

    // not debugging needed when we don't even see an output
    #[cfg(feature = "graphics")]
    if display {
        app.add_plugins(debug::debug_plugin);
    }

    app.add_plugins(navmesh_generation_plugin)
        .add_systems(
            Update,
            exit_app_system
                .run_if(in_state(ProcessingStep::Exit))
                .run_if(|exit: Res<EarlyExit>| exit.0),
        )
        .run();

    Ok(())
}

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        #[cfg(feature = "graphics")]
        Camera3d::default(),
        #[cfg(feature = "graphics")]
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
}

#[cfg(feature = "graphics")]
fn setup_wireframe(mut commands: Commands) {
    commands.insert_resource(WireframeConfig {
        global: true,
        ..default()
    });
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
