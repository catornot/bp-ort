use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_fly_camera::FlyCamera;
use oktree::prelude::*;
use std::{iter, ops::BitAnd, sync::Arc};

use crate::{
    ATTRIBUTE_PRIMATIVE_TYPE, ATTRIBUTE_UNIQUE_CONTENTS, CELL_SIZE, ChunkCells, DebugAmount,
    OFFSET, PrimitiveType, ProcessingStep, WireMe, WorldMesh,
    async_pathfinding::JobMarket,
    behavior::{self, Behavior, init_pathfinding},
};

pub type Octree32 = Octree<u32, TUVec3u32>;
pub struct Navmesh {
    pub navmesh_tree: Octree32,
    pub cell_size: f32,
}

#[derive(Resource)]
pub struct NavmeshRes(Behavior, JobMarket);

#[derive(Resource, Default)]
pub struct PathfindingPoints {
    building: [Option<Vec3>; 2],
    current: Option<[Vec3; 2]>,
}

#[derive(Component)]
struct PointsText;

#[derive(Component)]
struct MeshInfoText;

pub fn debug_plugin(app: &mut App) {
    app.add_systems(
        Update,
        (
            debug_world,
            debug_pathfinding.run_if(resource_exists::<NavmeshRes>),
            update_pos_text,
            add_pathfinding_points.run_if(input_just_pressed(KeyCode::Enter)),
            add_navmesh_resource
                .run_if(in_state(ProcessingStep::Done))
                .run_if(|res: Option<Res<NavmeshRes>>| res.is_none()),
            debug_contents,
        ),
    )
    .add_systems(Startup, setup_debug_ui)
    .init_resource::<PathfindingPoints>();
}

fn setup_debug_ui(mut commands: Commands) {
    // create our UI root node
    // this is the wrapper/container for the text
    let root = commands
        .spawn((
            ZIndex(i32::MAX),
            BackgroundColor(Color::BLACK.with_alpha(0.5)),
            TextLayout::default(),
            Node {
                // give it a dark background for readability
                // make it "always on top" by setting the Z index to maximum
                // we want it to be displayed over all other UI
                display: Display::Grid,
                position_type: PositionType::Absolute,
                // position it at the top-right corner
                // 1% away from the top window edge
                right: Val::Percent(1.),
                top: Val::Percent(1.),
                // set bottom/left to Auto, so it can be
                // automatically sized depending on the text
                bottom: Val::Auto,
                left: Val::Auto,
                // give it some padding for readability
                padding: UiRect::all(Val::Px(4.0)),
                min_width: Val::Px(100.),
                ..default()
            },
        ))
        .id();

    let text_bundles = (
        TextColor(Color::WHITE),
        TextFont {
            font_size: 16.0,
            ..default()
        },
    );

    let text_points = commands
        .spawn((
            PointsText,
            text_bundles.clone(),
            TextLayout::default(),
            Text::new("Points"),
        ))
        .with_child((text_bundles.clone(), TextSpan(" ".into())))
        .with_child((text_bundles.clone(), TextSpan("None".into())))
        .with_child((text_bundles.clone(), TextSpan(" ".into())))
        .with_child((text_bundles.clone(), TextSpan("None".into())))
        .id();

    let mesh_info = commands
        .spawn((
            MeshInfoText,
            text_bundles.clone(),
            TextLayout::default(),
            Text::new("Mesh"),
        ))
        .with_child((text_bundles.clone(), TextSpan(" ".into())))
        .with_child((text_bundles.clone(), TextSpan("N/A".into())))
        .with_child((text_bundles.clone(), TextSpan(" ".into())))
        .with_child((text_bundles.clone(), TextSpan("N/A".into())))
        .id();

    commands
        .entity(root)
        .add_children(&[text_points, mesh_info]);
}

fn update_pos_text(
    points: Res<PathfindingPoints>,
    mut query: Query<Entity, With<PointsText>>,
    mut writer: TextUiWriter,
) {
    for ent in &mut query {
        *writer.text(ent, 2) = points.building[0]
            .or_else(|| points.current.map(|points| points[0]))
            .map(|point| format!("{},{},{}", point.x, point.y, point.z))
            .unwrap_or_else(|| "None".to_owned());
        *writer.text(ent, 4) = points.building[1]
            .or_else(|| {
                points
                    .current
                    .map(|points| points[1])
                    .filter(|_| points.building[0].is_none())
            })
            .map(|point| format!("{},{},{}", point.x, point.y, point.z))
            .unwrap_or_else(|| "None".to_owned());
    }
}

fn debug_world(
    camera: Query<&Transform, (With<FlyCamera>, Without<WireMe>)>,
    debug_amount: Res<DebugAmount>,
    cells: Res<ChunkCells>,
    mut gizmos: Gizmos,
) -> Result<(), BevyError> {
    let origin = camera.single()?.translation;

    for pos in cells
        .collied_vec
        .iter()
        .map(|inter| {
            (UVec3::from_array(inter.cord).as_ivec3() - IVec3::splat(OFFSET)).as_vec3()
                * Vec3::splat(CELL_SIZE)
        })
        .filter(|pos| pos.distance(origin) < 500.)
    {
        gizmos.cuboid(
            Transform::from_translation(pos).with_scale(Vec3::splat(CELL_SIZE)),
            Color::srgba_u8(255, 0, 0, 255),
        );
    }

    if !debug_amount.octree {
        return Ok(());
    }

    for (center, scale) in cells
        .tree
        .iter_nodes()
        .map(|node| (node.aabb.center(), node.aabb.size()))
        .map(|(center, scale)| ([center.x, center.y, center.z], UVec3::splat(scale)))
        .map(|(center, scale)| {
            (
                (UVec3::from_array(center).as_ivec3() - IVec3::splat(OFFSET)).as_vec3()
                    * Vec3::splat(CELL_SIZE),
                (scale.as_vec3() * Vec3::splat(CELL_SIZE)),
            )
        })
        .filter(|(center, _)| center.distance(origin) < 500.)
    {
        gizmos.cuboid(
            Transform::from_translation(center).with_scale(scale),
            Color::srgba_u8(255, 255, 0, 255),
        );
    }

    Ok(())
}

fn debug_pathfinding(
    _debug_amount: Res<DebugAmount>,
    points: Res<PathfindingPoints>,
    mut navmesh: ResMut<NavmeshRes>,
    time: Res<Time>,
    gizmos: Gizmos,
) -> Result<(), BevyError> {
    if let Some(points) = points.current {
        let NavmeshRes(bt, job_market) = &mut *navmesh;
        behavior::run_behavior(bt, time.delta_secs_f64(), points, job_market, gizmos);
    }

    Ok(())
}

fn add_pathfinding_points(
    camera: Query<&Transform, (With<FlyCamera>, Without<WireMe>)>,
    mut points: ResMut<PathfindingPoints>,
) -> Result<(), BevyError> {
    let origin = camera.single()?.translation.trunc();

    if points.building[0].is_none() {
        _ = points.building[0].replace(origin);
        _ = points.building[1].take();
    } else if points.building[1].is_none() {
        _ = points.building[1].replace(origin);
    }

    if matches!(points.building, [Some(_), Some(_)]) {
        points.current = points.building[0].and_then(|point| Some([point, points.building[1]?]));
        points.building = default();
    }

    Ok(())
}

fn add_navmesh_resource(mut commands: Commands, cells: Res<ChunkCells>) {
    let navmesh = Arc::new(Navmesh {
        navmesh_tree: cells.tree.clone(),
        cell_size: CELL_SIZE,
    });
    commands.insert_resource(NavmeshRes(
        init_pathfinding(Arc::clone(&navmesh)),
        JobMarket::new(navmesh),
    ));
}

fn debug_contents(
    mut ray_cast: MeshRayCast,
    camera: Query<&Transform, (With<FlyCamera>, Without<WorldMesh>)>,
    world_meshes: Query<&Mesh3d, (With<WorldMesh>, Without<FlyCamera>)>,
    meshes: Res<Assets<Mesh>>,
    mut text: Query<Entity, With<MeshInfoText>>,
    mut writer: TextUiWriter,
    mut gizmos: Gizmos,
) -> Result<(), BevyError> {
    let Transform {
        translation,
        rotation,
        scale: _,
    } = *camera.single()?;

    let Some(result) = ray_cast
        .cast_ray(
            Ray3d::new(
                translation,
                // Dir3::new(
                //     Transform::from_rotation(rotation)
                //         .transform_point(Vec3::X)
                //         .normalize(),
                // )
                // .unwrap_or(),
                Dir3::X,
            ),
            &MeshRayCastSettings::default(),
        )
        .first()
    else {
        bevy::log::warn!("couldn't get anything in this ray cast odd");
        return Ok(());
    };

    gizmos.line(
        translation + Vec3::new(0., -10., 0.),
        result.1.point,
        Color::srgb_from_array([1.0, 1.0, 1.0]),
    );

    if let Some(mesh) = world_meshes
        .get(result.0)
        .ok()
        .and_then(|mesh| meshes.get(mesh.id()))
    {
        let ty = mesh
            .attribute(ATTRIBUTE_PRIMATIVE_TYPE)
            .and_then(|values| <[u8; 4]>::try_from(values.get_bytes().get(0..4)?).ok())
            .map(u32::from_ne_bytes)
            .map(PrimitiveType::try_from)
            .and_then(|maybe_err| maybe_err.ok())
            .map(|ty| match ty {
                PrimitiveType::Brush => "Brush",
                PrimitiveType::Tricoll => "Tricoll",
                PrimitiveType::Prop => "Prop",
            })
            .unwrap_or("N/A");

        const FLAGS: [(&str, i32); 31] = [
            ("EMPTY\n", 0x00),
            ("SOLID\n", 0x01),
            ("WINDOW\n", 0x02),
            ("AUX\n", 0x04),
            ("GRATE\n", 0x08),
            ("SLIME\n", 0x10),
            ("WATER\n", 0x20),
            ("WINDOW_NO_COLLIDE\n", 0x40),
            ("ISOPAQUE\n", 0x80),
            ("TEST_FOG_VOLUME\n", 0x100),
            ("UNUSED_1\n", 0x200),
            ("BLOCK_LIGHT\n", 0x400),
            ("TEAM_1\n", 0x800),
            ("TEAM_2\n", 0x1000),
            ("IGNORE_NODRAW_OPAQUE\n", 0x2000),
            ("MOVEABLE\n", 0x4000),
            ("PLAYER_CLIP\n", 0x10000),
            ("MONSTER_CLIP\n", 0x20000),
            ("BRUSH_PAINT\n", 0x40000),
            ("BLOCK_LOS\n", 0x80000),
            ("NO_CLIMB\n", 0x100000),
            ("TITAN_CLIP\n", 0x200000),
            ("BULLET_CLIP\n", 0x400000),
            ("UNUSED_5\n", 0x800000),
            ("ORIGIN\n", 0x1000000),
            ("MONSTER\n", 0x2000000),
            ("DEBRIS\n", 0x4000000),
            ("DETAIL\n", 0x8000000),
            ("TRANSLUCENT\n", 0x10000000),
            ("LADDER\n", 0x20000000),
            ("HITBOX\n", 0x40000000),
        ];

        let contents = mesh
            .attribute(ATTRIBUTE_UNIQUE_CONTENTS)
            .and_then(|values| <[u8; 4]>::try_from(values.get_bytes().get(0..4)?).ok())
            .map(i32::from_ne_bytes)
            .unwrap_or(0);

        let contents = FLAGS
            .iter()
            .zip([contents; FLAGS.len()].iter())
            .filter_map(|(flag, contents)| flag.1.bitand(*contents).eq(&flag.1).then_some(flag.0))
            .collect::<String>();

        for ent in &mut text {
            *writer.text(ent, 4) = ty.to_string();
            *writer.text(ent, 2) = contents.to_string();
        }
    }

    Ok(())
}
