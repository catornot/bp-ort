use bevy::{input::common_conditions::input_just_pressed, prelude::*};
use bevy_fly_camera::FlyCamera;
use oktree::prelude::*;
use std::sync::Arc;

use crate::{
    CELL_SIZE, ChunkCells, DebugAmount, OFFSET, ProcessingStep, WireMe,
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

    commands.entity(root).add_children(&[text_points]);
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
    let origin = camera.single()?.translation;

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
    let (min, max) = (
        cells
            .collied_vec
            .iter()
            .flat_map(|cell| cell.cord)
            .min()
            .unwrap_or(0),
        cells
            .collied_vec
            .iter()
            .flat_map(|cell| cell.cord)
            .max()
            .unwrap_or_else(|| unreachable!()),
    );

    let mut octree: Octree32 = Octree::from_aabb_with_capacity(
        dbg!(Aabb::from_min_max(
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
        )),
        cells.collied_vec.len(),
    );

    let mut err = String::new();
    cells
        .collied_vec
        .iter()
        .map(|cell| cell.cord)
        // swizzle here too
        .for_each(|pos| {
            _ = octree
                .insert(TUVec3u32::new(pos[0], pos[2], pos[1]))
                .inspect_err(|thiserr| err = thiserr.to_string());
        });

    let navmesh = Arc::new(Navmesh {
        navmesh_tree: octree,
        cell_size: CELL_SIZE,
    });
    commands.insert_resource(NavmeshRes(
        init_pathfinding(Arc::clone(&navmesh)),
        JobMarket::new(navmesh),
    ));
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
