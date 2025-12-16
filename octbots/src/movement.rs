use bevy_math::{bounding::RayCast3d, prelude::*};
use bonsai_bt::Status;
use oktree::prelude::*;
use parry3d::shape::Capsule;
use parry3d::{self, query::RayCast};
use rrplug::{
    bindings::class_types::{cbaseentity::CBaseEntity, cplayer::CPlayer},
    prelude::*,
};
use shared::cmds_helper::CUserCmdHelper;
use shared::{
    bindings::{
        Action as MoveAction, CGameTrace, CTraceFilterSimple, Contents, Ray, TraceCollisionGroup,
        VectorAligned,
    },
    utils::{lookup_ent, nudge_type},
};
use std::{f32::consts::PI, mem::MaybeUninit, ops::Not};

use crate::behavior::BotBrain;
use crate::pathfinding::AreaCost;
use crate::{
    behavior::BotAction,
    loader::{Navmesh, NavmeshStatus},
    nav_points::{tuvec_to_vector3, vector3_to_tuvec, NavPoint},
    pathfinding::get_neighbors_h,
};

#[derive(Debug, Clone)]
pub struct Movement {
    pub next_wall_point: Option<Vector3>,
    pub next_point_override: Option<Vector3>,
    /// lock on view from the enemy targeting system
    /// when false the movement system can control the view angles
    pub view_lock: bool,
    /// clamped angles for the targeting sytem when wallrunning
    /// maybe should be a [[Option<f32>]]
    pub clamped_view: f32,

    // juump
    pub jump_tick: u32,
    pub vault_tick: u32,
    pub down_tick: u32,

    // area cost
    // should make some wrapper around this that can be shared
    pub area_cost: AreaCost,
    pub last_path_points: Vec<NavPoint>,
    pub last_point_reached_delta: f32,
}

#[derive(Debug, Clone)]
pub enum MovementAction {
    CanMove,
    CheckReachability,
    Move,
    IsJump,
    Jump,
    IsFenceHop,
    TryMountFence,
    IsCrawling,
    Crawl,
    IsWallRun,
    WallRun,
    IsGoingDown,
    GoDownBetter,
    FinishMove,
}

impl From<MovementAction> for BotAction {
    fn from(val: MovementAction) -> Self {
        BotAction::Movement(val)
    }
}

pub fn run_movement(
    movement: &MovementAction,
    brain: &mut BotBrain,
    bot: &CPlayer,
    helper: &CUserCmdHelper,
) -> (Status, f64) {
    match movement {
        MovementAction::CanMove => {
            if brain.path.is_empty() {
                (Status::Failure, 0.)
            } else {
                let debug = crate::ENGINE_INTERFACES.wait().debug_overlay;
                unsafe {
                    debug.AddLineOverlay(
                        &brain.origin,
                        brain
                            .path
                            .front()
                            .map(AsRef::as_ref)
                            .unwrap_or(&Vector3::ZERO),
                        0,
                        100,
                        150,
                        true,
                        0.01,
                    )
                }
                (Status::Success, 0.)
            }
        }
        MovementAction::CheckReachability => {
            let build_ray = |v1: Vector3, v2: Vector3| Ray {
                start: VectorAligned { vec: v1, w: 0. },
                delta: VectorAligned {
                    vec: v2 - v1,
                    w: 0.,
                },
                offset: VectorAligned {
                    vec: Vector3::new(0., 0., 0.),
                    w: 0.,
                },
                unk3: 0.,
                unk4: 0,
                unk5: 0.,
                unk6: 1103806595072,
                unk7: 0.,
                is_ray: true,
                is_swept: false,
                is_smth: false,
                flags: 0,
                unk8: 0,
            };
            if let Some(point) = brain.path.front()
                && let Some(navmesh) = brain.navmesh.try_read()
                && let NavmeshStatus::Loaded(_) = &navmesh.navmesh
                && unsafe {
                    let mut result_low = MaybeUninit::<CGameTrace>::zeroed();
                    let mut result_high = MaybeUninit::<CGameTrace>::zeroed();
                    const HIGH_OFFSET: Vector3 = Vector3::new(0., 0., 100.);
                    const LOW_OFFSET: Vector3 = Vector3::new(0., 0., 40.);
                    let ray_low =
                        build_ray(brain.abs_origin + LOW_OFFSET, point.as_vec() + LOW_OFFSET);
                    let ray_high =
                        build_ray(brain.abs_origin + HIGH_OFFSET, point.as_vec() + HIGH_OFFSET);

                    let filter: *const CTraceFilterSimple = &CTraceFilterSimple {
                        vtable: helper.sv_funcs.simple_filter_vtable,
                        unk: 0,
                        pass_ent: nudge_type::<&CBaseEntity>(bot),
                        should_hit_func: std::ptr::null(),
                        collision_group: TraceCollisionGroup::None as i32,
                    };

                    (helper.engine_funcs.trace_ray_filter)(
                        (*helper.sv_funcs.ctraceengine) as *const libc::c_void,
                        &ray_high,
                        Contents::SOLID as u32
                            | Contents::MOVEABLE as u32
                            | Contents::WINDOW as u32
                            | Contents::MONSTER as u32
                            | Contents::GRATE as u32
                            | Contents::PLAYER_CLIP as u32,
                        filter.cast(),
                        result_high.as_mut_ptr(),
                    );

                    (helper.engine_funcs.trace_ray_filter)(
                        (*helper.sv_funcs.ctraceengine) as *const libc::c_void,
                        &ray_low,
                        Contents::SOLID as u32
                            | Contents::MOVEABLE as u32
                            | Contents::WINDOW as u32
                            | Contents::MONSTER as u32
                            | Contents::GRATE as u32
                            | Contents::PLAYER_CLIP as u32,
                        filter.cast(),
                        result_low.as_mut_ptr(),
                    );

                    result_low
                        .assume_init()
                        .fraction
                        .max(result_high.assume_init().fraction)
                } != 1.0
            {
                *brain.m.area_cost.entry(point.as_point()).or_default() += 100.;
                // also add to the last 5 points
                brain
                    .m
                    .last_path_points
                    .iter()
                    .rev()
                    .take(5)
                    .for_each(|point| {
                        *brain.m.area_cost.entry(point.as_point()).or_default() += 100.;
                    });

                brain.m.last_point_reached_delta = 0.;
                brain.path.clear();
                brain.needs_new_path = true;
                brain.path_receiver = None; // remove any future paths
                (Status::Failure, 0.)
            } else if let Some(point) = brain.path.front()
                && brain.m.last_point_reached_delta > 5.
            {
                *brain.m.area_cost.entry(point.as_point()).or_default() += 100.;
                // also add to the last 5 points
                brain
                    .m
                    .last_path_points
                    .iter()
                    .rev()
                    .take(5)
                    .for_each(|point| {
                        *brain.m.area_cost.entry(point.as_point()).or_default() += 100.;
                    });

                brain.m.last_point_reached_delta = 0.;
                brain.path.clear();
                brain.needs_new_path = true;
                brain.path_receiver = None; // remove any future paths
                (Status::Failure, 0.)
            } else {
                (Status::Success, 0.)
            }
        }
        MovementAction::Move => '_move: {
            // move towards wall point if we have to wallrun
            let Some(target) = brain
                .m
                .next_wall_point
                .or(brain.m.next_point_override)
                .or_else(|| brain.path.front().map(AsRef::<Vector3>::as_ref).copied())
            else {
                break '_move (Status::Failure, 0.);
            };

            const TURN_RATE: f32 = PI / 3.;
            let angle = (target.y - brain.origin.y).atan2(target.x - brain.origin.x);
            brain.next_cmd.world_view_angles.y = angle
                .clamp(angle - TURN_RATE, angle + TURN_RATE)
                .to_degrees()
                * brain.m.view_lock.not() as i32 as f32
                + brain.angles.y * brain.m.view_lock as i32 as f32;
            brain.next_cmd.world_view_angles.x = 0.;

            let forward_vector = Vec2::new(
                brain.next_cmd.world_view_angles.y.to_radians().cos(),
                brain.next_cmd.world_view_angles.y.to_radians().sin(),
            );

            let angle = -forward_vector.angle_to(Vec2::new(
                brain.origin.x - target.x,
                brain.origin.y - target.y,
            ));

            let move2d = -Vec2::new(angle.cos(), angle.sin());

            let move_ = Vector3::from(move2d.extend(brain.next_cmd.move_.z).to_array());

            let debug = crate::ENGINE_INTERFACES.wait().debug_overlay;
            unsafe {
                debug.AddLineOverlay(
                    &brain.origin,
                    &(brain.origin + move_ * Vector3::new(30., 30., 30.)),
                    200,
                    100,
                    150,
                    true,
                    0.01,
                )
            }

            if brain.m.next_wall_point.is_some() || brain.m.next_point_override.is_some() {
                unsafe {
                    debug.AddLineOverlay(
                        &brain.origin,
                        &target,
                        255,
                        brain.m.next_point_override.is_some() as i32 * 255,
                        150,
                        true,
                        0.01,
                    )
                }
            }

            brain.next_cmd.move_ = move_;
            brain.next_cmd.buttons |= MoveAction::Speed as u32;

            (Status::Success, 0.)
        }
        MovementAction::IsJump => match brain.path.front() {
            Some(point)
                if point.as_vec().z
                    > brain.abs_origin.z
                        + 50.
                        + brain
                            .m.next_wall_point
                            .and_then(|_| brain.navmesh.try_read())
                            .map(|navmesh| navmesh.cell_size)
                            .unwrap_or_default() // add more leway when wallrunning
                    || bot.m_vecAbsVelocity == Vector3::ZERO =>
            {
                (Status::Success, 0.)
            }
            Some(_) => (Status::Failure, 0.),
            None => (Status::Failure, 0.),
        },
        MovementAction::Jump => {
            brain.m.jump_tick += 1;
            if bot.m_vecAbsVelocity.z - 5. <= 0.
                || lookup_ent(bot.m_hGroundEntity, helper.sv_funcs).is_some()
            {
                let jummp = (brain.m.jump_tick % 4 < 2) as u32;
                brain.next_cmd.move_.z = jummp as f32;
                brain.next_cmd.buttons |= MoveAction::Jump as u32 * jummp;
                (Status::Success, 0.)
            } else {
                brain.next_cmd.move_.z = 0.;
                brain.next_cmd.buttons &= !(MoveAction::Jump as u32);
                (Status::Failure, 0.)
            }
        }
        MovementAction::IsFenceHop => match brain.path.front() {
            Some(point)
                if (point.z > brain.abs_origin.z + 10.
                    || (bot.m_vecAbsVelocity.x.abs() < 0.01
                        && bot.m_vecAbsVelocity.y.abs() < 0.01))
                    && brain.m.next_wall_point.is_none() =>
            {
                (Status::Success, 0.)
            }
            Some(_) => (Status::Failure, 0.),
            None => (Status::Failure, 0.),
        },
        MovementAction::TryMountFence => {
            brain.m.vault_tick += 1;

            let fence_check =
                |navmesh: &Navmesh, octtree: &Octree<u32, TUVec3u32>, dir: Vector3| {
                    octtree
                        .ray_cast(&RayCast3d::new(
                            Vec3::new(brain.abs_origin.x, brain.abs_origin.y, brain.abs_origin.z)
                                / Vec3::splat(navmesh.cell_size),
                            Dir3A::new_unchecked(Vec3::new(dir.x, dir.y, 0.).normalize().into()),
                            navmesh.cell_size,
                        ))
                        .element
                        .and_then(|element| octtree.get_element(element))
                        .copied()
                        .map(|pos| TUVec3::new(pos.0.x, pos.0.y, pos.0.z + 1))
                };
            if brain.m.vault_tick % 12 < 5
                && bot.m_vecAbsVelocity.x + bot.m_vecAbsVelocity.y <= 5.
                // not sure about the ground check
                && lookup_ent(bot.m_hGroundEntity, helper.sv_funcs).is_some()
                && let Some(dir) = brain
                    .path
                    .front()
                    .map(|target| brain.abs_origin - **target )
                && let Some(navmesh) = brain.navmesh.try_read()
                && let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh
                && let Some(element) = fence_check(&navmesh, octtree, dir)
                && octtree.get(&element).is_none()
            {
                brain.next_cmd.move_.z = 1.;
                brain.next_cmd.buttons |= MoveAction::Jump as u32;
                (Status::Success, 0.)
            } else {
                brain.next_cmd.move_.z = 0.;
                brain.next_cmd.buttons &= !(MoveAction::Jump as u32);
                (Status::Failure, 0.)
            }
        }
        MovementAction::IsCrawling => match (brain.path.front(), brain.navmesh.try_read()) {
            (Some(point), Some(navmesh))
                if brain.m.next_wall_point.is_none()
                    && let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh
                    && let point = vector3_to_tuvec(
                        navmesh.cell_size,
                        **point + Vector3::new(0., 0., navmesh.cell_size),
                    )
                    .0
                    && (octtree.get(&point).is_some()
                        || octtree
                            .get(&TUVec3::new(point.x, point.y, point.z + 1))
                            .is_some()) =>
            {
                (Status::Success, 0.)
            }
            _ => (Status::Failure, 0.),
        },
        MovementAction::Crawl => {
            brain.next_cmd.buttons |= MoveAction::Duck as u32;

            (Status::Success, 0.)
        }
        MovementAction::IsWallRun => {
            let is_wallrun_point = |point: &NavPoint| {
                brain
                    .path
                    .get(1)
                    .map(|next_point| (next_point.as_point(), point.as_point()))
                    .filter(|(next_point, point)| {
                        (next_point.0.x == point.0.x || next_point.0.y == point.0.y)
                            && next_point.0.z == point.0.z
                    })
                    .is_some()
            };

            // anything below 1 distance off the ground isn't wallrunnable
            // we find if the next point is wallrunable that means that this mean isn't some random spot where we are passing near the wall
            match (brain.path.front(), brain.navmesh.try_read()) {
                (Some(point), Some(ref navmesh))
                    if point.as_distance() > 2
                        && point.z + navmesh.cell_size >= brain.abs_origin.z
                        && let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh
                        && let Some(wall_point) = get_neighbors_h(*point.as_ref(), octtree)
                            .find_map(|(point, is_empty)| is_empty.not().then_some(point))
                        // check next path 
                        && is_wallrun_point(point) =>
                {
                    let diff = dbg!((Vec3::new(
                        point.as_point().0.x as f32 - wall_point.0.x as f32,
                        point.as_point().0.y as f32 - wall_point.0.y as f32,
                        point.as_point().0.z as f32 - wall_point.0.z as f32,
                    )
                    .abs()
                    .min(Vec3::splat(1.))
                    .max(Vec3::ZERO)
                        - Vec3::splat(1.))
                    .abs());

                    brain.m.next_wall_point = unsafe {
                        let mut result = MaybeUninit::<CGameTrace>::zeroed();
                        let mut ray = MaybeUninit::<Ray>::zeroed().assume_init(); // all zeros is correct for Ray
                        ray.unk6 = 0;
                        let wall_pos = tuvec_to_vector3(navmesh.cell_size, wall_point);
                        (helper.sv_funcs.create_trace_hull)(
                            &mut ray,
                            point.as_ref(),
                            &wall_pos,
                            &Vector3::new(
                                -navmesh.cell_size * diff.x,
                                -navmesh.cell_size * diff.y,
                                -navmesh.cell_size * diff.z,
                            ),
                            &Vector3::new(
                                navmesh.cell_size * diff.x,
                                navmesh.cell_size * diff.y,
                                navmesh.cell_size * diff.z,
                            ),
                        );

                        let filter: *const CTraceFilterSimple = &CTraceFilterSimple {
                            vtable: helper.sv_funcs.simple_filter_vtable,
                            unk: 0,
                            pass_ent: std::ptr::null(),
                            should_hit_func: std::ptr::null(),
                            collision_group: TraceCollisionGroup::None as i32,
                        };

                        ray.is_smth = false;

                        (helper.engine_funcs.trace_ray_filter)(
                            (*helper.sv_funcs.ctraceengine) as *const libc::c_void,
                            &ray,
                            Contents::SOLID as u32
                                | Contents::MOVEABLE as u32
                                | Contents::WINDOW as u32
                                | Contents::MONSTER as u32
                                | Contents::GRATE as u32
                                | Contents::PLAYER_CLIP as u32,
                            filter.cast(),
                            result.as_mut_ptr(),
                        );

                        Some(result.assume_init().end_pos)
                    };

                    (Status::Success, 0.)
                }
                _ => (Status::Failure, 0.),
            }
        }
        MovementAction::WallRun => (Status::Success, 0.),
        MovementAction::IsGoingDown => match brain.path.front() {
            // should maybe check if the next points are not above
            Some(point)
                if let Some(navmesh) = brain.navmesh.try_read()
                    && point.as_vec().z + navmesh.cell_size / 2. < brain.abs_origin.z
                    && point.as_distance() > 1 =>
            {
                (Status::Success, 0.)
            }
            Some(_) => (Status::Failure, 0.),
            None => (Status::Failure, 0.),
        },
        MovementAction::GoDownBetter => {
            brain.m.down_tick += 1;
            let distance2d = |p: TUVec3<u32>, v: Vector3| {
                ((p.x as f32 - v.x).powi(2) + ((p.y as f32 - v.y).powi(2))).sqrt()
            };
            // INFO: this could actually help sometimes, by checking if there is anything between the bot and the go down better point
            // currently doesn't work tho
            // let any_obstructions =
            //     |start: TUVec3u32, end: TUVec3u32, octtree: &Octree32, navmesh: &Navmesh| {
            //         octtree
            //             .ray_cast(&RayCast3d::new(
            //                 UVec3::new(start.0.x, start.0.y, start.0.z).as_vec3a(),
            //                 Dir3A::new_unchecked((UVec3::new(start.0.x, start.0.y, start.0.z).as_vec3a() - UVec3::new(end.0.x, end.0.y, end.0.z).as_vec3a()).normalize()),
            //                 navmesh.cell_size,
            //             ))
            //             .element
            //             .and_then(|element| octtree.get_element(element))
            //         .is_some()
            //     };
            let get_drop_point = |point: TUVec3u32, octtree| {
                get_neighbors_h(point, octtree)
                    .filter_map(|(point, is_empty)| is_empty.then_some(point))
                    // .filter(|potential_point| !any_obstructions(point.as_point(), *potential_point, octtree, &navmesh) )
                    .map(|point| {
                        (
                            point,
                            get_neighbors_h(point, octtree)
                                .filter(|(_, is_empty)| !*is_empty)
                                .count(),
                        )
                    })
                    .reduce(|l, r| {
                        // if the amount of walls is the same check for distance
                        if l.1 == r.1 {
                            if distance2d(l.0 .0, brain.abs_origin)
                                < distance2d(r.0 .0, brain.abs_origin)
                            {
                                l
                            } else {
                                r
                            }
                        } else if l.1 < r.1 {
                            l
                        } else {
                            r
                        }
                    })
            };
            if let Some(point) = brain.path.front()
                && let Some(navmesh) = brain.navmesh.try_read()
                // this just breaks this system :(
                // && distance2d(point.as_point().0, brain.abs_origin) < navmesh.cell_size * 2.  // check if we are not able to fall
                && let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh
                && let Some(point_offset) = get_drop_point(point.as_point(), octtree)
            {
                if brain.m.down_tick > 16 {
                    // the worse way of getting a unit vector
                    let offset = (tuvec_to_vector3(navmesh.cell_size, point_offset.0)
                        - point.as_vec())
                        * Vector3::new(1., 1., 1.);

                    // let angle = (point.y - brain.origin.y).atan2(point.x - brain.origin.x);
                    // let offset = Vector3::new(angle.cos(), angle.sin(), 0.)
                    //     * Vector3::new(navmesh.cell_size, navmesh.cell_size, navmesh.cell_size);

                    brain.next_cmd.move_.z = 0.;
                    brain.m.next_point_override = Some(point.as_vec() + offset);
                }
                (Status::Success, 0.)
            } else {
                brain.m.down_tick = 0;
                (Status::Failure, 0.)
            }
        }
        MovementAction::FinishMove => '_move: {
            let Some(next_point) = brain.path.front() else {
                break '_move (Status::Failure, 0.);
            };
            let Some(navmesh) = brain.navmesh.try_read() else {
                break '_move (Status::Failure, 0.);
            };

            let hitbox: Capsule = Capsule::new_z(
                50. + brain
                    .m
                    .next_wall_point
                    .and_then(|_| brain.navmesh.try_read())
                    .map(|navmesh| navmesh.cell_size * 2.)
                    .unwrap_or_default(), // add more leway when wallrunning
                25.,
            )
            .transform_by(&[brain.origin.x, brain.origin.y, brain.origin.z].into());
            let is_in_hitbox = |target: &Vector3| {
                hitbox.intersects_local_ray(
                    &parry3d::query::Ray::new(
                        [target.x, target.y, target.z].into(),
                        [0., 0., 1.].into(),
                    ),
                    0.01,
                )
            };

            // increment with tick interval for reachability tests
            brain.m.last_point_reached_delta += helper.globals.absoluteFrameTime;

            // look 20 points ahead for when a bot overshoots points
            // TODO: figure if restricting z is actaully a good idea
            // this begin restriting point skipping to one point above or less and equals based on z pos
            if brain
                .path
                .iter()
                .take(20)
                .map(AsRef::as_ref)
                .filter(|pos: &&Vector3| pos.z <= next_point.as_vec().z + navmesh.cell_size)
                .any(is_in_hitbox)
                && let Some(nav_point) = brain.path.pop_front()
            {
                brain.m.last_path_points.push(nav_point);
                brain.m.last_point_reached_delta = 0.; // reset delta
            }

            brain.m.next_wall_point = None;
            brain.m.next_point_override = None;
            brain.needs_new_path = brain.path.len() < 3;

            (Status::Success, 0.)
        }
    }
}
