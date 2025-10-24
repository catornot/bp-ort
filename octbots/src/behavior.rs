use bevy_math::{bounding::RayCast3d, prelude::*};
use bonsai_bt::{
    Action,
    Behavior::{AlwaysSucceed, Select, WhenAny},
    Event, Sequence, Status, UpdateArgs, BT, RUNNING,
};
use itertools::Itertools;
use oktree::prelude::*;
use parking_lot::RwLock;
use parry3d::shape::Capsule;
use parry3d::{self, query::RayCast};
use rrplug::{
    bindings::class_types::{client::CClient, cplayer::CPlayer},
    prelude::*,
};
use shared::{
    bindings::{Action as MoveAction, CUserCmd, TraceResults, SERVER_FUNCTIONS},
    cmds_helper::CUserCmdHelper,
    utils::{lookup_ent, nudge_type},
};
use std::{
    collections::{HashMap, VecDeque},
    f32::consts::PI,
    mem::MaybeUninit,
    ops::Not,
    sync::{Arc, LazyLock},
};

use crate::{
    async_pathfinding::PathReceiver,
    loader::{Navmesh, NavmeshStatus},
    nav_points::{get_neighbors_h, tuvec_to_vector3, vector3_to_tuvec, NavPoint},
};

static BEHAVIOR: LazyLock<RwLock<HashMap<u16, BT<BotAction, BotBrain>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

struct BotBrain {
    navmesh: Arc<RwLock<Navmesh>>,
    current_target: Option<i32>,
    path_receiver: Option<PathReceiver>,
    path_next_request: f32,
    path: VecDeque<NavPoint>,
    origin: Vector3,
    angles: Vector3,
    abs_origin: Vector3,
    next_cmd: CUserCmd,
    next_wall_point: Option<Vector3>,
    last_alive_state: bool,
    needs_new_path: bool,
    /// lock on view from the enemy targeting system
    /// when false the movement system can control the view angles
    view_lock: bool,
    /// clamped angles for the targeting sytem when wallrunning
    /// maybe should be a [[Option<f32>]]
    clamped_view: f32,
}

#[derive(Debug, Clone)]
pub enum BotAction {
    RenderPath,
    FindTarget,
    FindPath,
    CheckNavmesh,
    IsDead,
    DeadState,
    CanMove,
    Move,
    IsJump,
    Jump(i32),
    IsFenceHop,
    TryMountFence(i32),
    IsCrawling,
    Crawl,
    IsWallRun,
    WallRun,
    FinishMove,
}

pub fn drop_behaviors() {
    BEHAVIOR.write().clear();
}

pub extern "C" fn init_bot(edict: u16, client: &CClient) {
    let dead_state = Sequence(vec![
        Action(BotAction::IsDead),
        Action(BotAction::DeadState),
    ]);

    let target_moving = Sequence(vec![
        Action(BotAction::CanMove),
        Action(BotAction::Move),
        AlwaysSucceed(Box::new(Select(vec![
            Sequence(vec![
                Action(BotAction::IsWallRun),
                Action(BotAction::WallRun),
            ]),
            Sequence(vec![Action(BotAction::IsJump), Action(BotAction::Jump(0))]),
            Sequence(vec![
                Action(BotAction::IsCrawling),
                Action(BotAction::Crawl),
            ]),
            Sequence(vec![
                Action(BotAction::IsFenceHop),
                Action(BotAction::TryMountFence(0)),
            ]),
        ]))),
        Action(BotAction::FinishMove),
    ]);

    let target_tracking = Sequence(vec![
        Action(BotAction::FindTarget),
        Action(BotAction::RenderPath),
        WhenAny(vec![Action(BotAction::FindPath), dead_state, target_moving]),
    ]);

    let routine = Sequence(vec![Action(BotAction::CheckNavmesh), target_tracking]);

    log::info!(
        "init bot {} with {edict}",
        shared::utils::get_c_char_array_lossy(&client.m_szServerName)
    );
    BEHAVIOR.write().entry(edict).insert_entry(BT::new(
        routine,
        BotBrain {
            navmesh: Arc::clone(&crate::PLUGIN.wait().navmesh),
            current_target: None,
            next_cmd: CUserCmd::init_default(SERVER_FUNCTIONS.wait()),
            last_alive_state: false,
            path_receiver: None,
            path: VecDeque::new(),
            path_next_request: 0.,
            origin: Vector3::ZERO,
            angles: Vector3::ZERO,
            abs_origin: Vector3::ZERO,
            next_wall_point: None,
            needs_new_path: true,
            view_lock: false,
            clamped_view: 0.,
        },
    ));
}

pub extern "C" fn wallpathfining_bots(helper: &CUserCmdHelper, bot: &mut CPlayer) -> CUserCmd {
    let mut behavior_static = BEHAVIOR.write();
    let Some(bt) = behavior_static.get_mut(&(bot.pl.index as u16)) else {
        return CUserCmd::new_empty(helper);
    };

    let mut v = Vector3::ZERO;
    unsafe {
        (helper.sv_funcs.calc_absolute_velocity)(
            nudge_type::<&CPlayer>(bot),
            &nudge_type::<*const CPlayer>(bot),
            0,
            0,
        );
        (helper.sv_funcs.calc_origin)(
            nudge_type::<&CPlayer>(bot),
            &nudge_type::<*const CPlayer>(bot),
            0,
            0,
        );
    };
    bt.blackboard_mut().next_cmd = CUserCmd::new_empty(helper);
    bt.blackboard_mut().origin = unsafe { *bot.get_origin(&mut v) };
    bt.blackboard_mut().angles = unsafe { *bot.eye_angles(&mut v) };
    bt.blackboard_mut().abs_origin = bot.m_vecAbsOrigin;

    let is_alive = bot.m_lifeState == 0;
    if is_alive != bt.blackboard().last_alive_state {
        bt.reset_bt();
        bt.blackboard_mut().last_alive_state = is_alive;
    }

    let e = Event::from(UpdateArgs { dt: 0. });

    bt.tick(&e, &mut |args, brain| match args.action {
        BotAction::FindTarget => 'target: {
            let Some(current_target) = (0..helper.globals.maxPlayers)
                .flat_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_ref() })
                .filter(|other| other.pl.index != bot.pl.index)
                .map(|other| other.pl.index)
                .next()
            else {
                break 'target (Status::Failure, 0.);
            };
            brain.current_target = Some(current_target);

            (Status::Success, 0.)
        }
        BotAction::CheckNavmesh => {
            if matches!(brain.navmesh.read().navmesh, NavmeshStatus::Loaded(_)) {
                (Status::Success, 0.)
            } else {
                (Status::Failure, 0.)
            }
        }
        BotAction::FindPath => 'path: {
            let navmesh = brain.navmesh.read();
            let Some(other_player) = (unsafe {
                (helper.sv_funcs.get_player_by_index)(brain.current_target.unwrap_or(1)).as_ref()
            }) else {
                break 'path (Status::Failure, 0.);
            };

            let NavmeshStatus::Loaded(_navmesh_tree) = &navmesh.navmesh else {
                break 'path (Status::Failure, 0.);
            };

            if !brain.needs_new_path {
                break 'path RUNNING;
            }

            let mut v = Vector3::ZERO;
            let start = brain
                .path
                .back()
                .map(AsRef::as_ref)
                .copied()
                .unwrap_or(brain.origin);
            let end = unsafe { *other_player.get_origin(&mut v) };

            if let None = brain.path_receiver.as_ref()
                && brain.path_next_request < helper.globals.curTime
            {
                brain.path_receiver = crate::PLUGIN.wait().job_market.find_path(start, end);
            }

            let status = if let Some(path_receiver) = brain.path_receiver.as_ref() {
                match path_receiver.try_recv() {
                    Ok(Some(path)) => {
                        path.into_iter().for_each(|v| brain.path.push_back(v));

                        (Status::Success, 0.)
                    }
                    Ok(None) => {
                        brain.path_next_request = helper.globals.curTime + 0.1;
                        (Status::Failure, 0.)
                    }
                    Err(flume::TryRecvError::Disconnected) => (Status::Failure, 0.),
                    Err(flume::TryRecvError::Empty) => RUNNING,
                }
            } else {
                (Status::Failure, 0.)
            };

            if status != RUNNING {
                brain.path_receiver.take();
            }
            status
        }
        BotAction::RenderPath => 'render: {
            if brain.path.is_empty() {
                break 'render (Status::Success, 0.);
            }

            let debug = crate::ENGINE_INTERFACES.wait().debug_overlay;
            brain
                .path
                .iter()
                .cloned()
                .tuple_windows()
                .for_each(|(p1, p2)| unsafe {
                    debug.AddLineOverlay(&*p1, &*p2, 0, 255, 0, true, 0.5)
                });

            (Status::Success, 0.)
        }
        BotAction::CanMove => {
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
        BotAction::Move => '_move: {
            // move towards wall point if we have to wallrun
            let Some(target) = brain
                .next_wall_point
                .take()
                .or_else(|| brain.path.front().map(AsRef::<Vector3>::as_ref).copied())
            else {
                break '_move (Status::Failure, 0.);
            };

            const TURN_RATE: f32 = PI / 3.;
            let angle = (target.y - brain.origin.y).atan2(target.x - brain.origin.x);
            brain.next_cmd.world_view_angles.y = angle
                .clamp(angle - TURN_RATE, angle + TURN_RATE)
                .to_degrees()
                * brain.view_lock.not() as i32 as f32
                + brain.angles.y * brain.view_lock as i32 as f32;
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

            let move_ = Vector3::from(move2d.extend(0.).to_array());

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

            brain.next_cmd.move_ = move_;
            brain.next_cmd.buttons |= MoveAction::Speed as u32;

            (Status::Success, 0.)
        }
        BotAction::IsJump => match brain.path.front() {
            Some(point)
                if point.as_vec().z > brain.abs_origin.z + 50.
                    || bot.m_vecAbsVelocity == Vector3::ZERO =>
            {
                (Status::Success, 0.)
            }
            Some(_) => (Status::Failure, 0.),
            None => (Status::Failure, 0.),
        },
        BotAction::Jump(mut frames) => {
            frames += 1;

            if frames % 6 < 3
                && (bot.m_vecAbsVelocity.z - 5. <= 0.
                    || lookup_ent(bot.m_hGroundEntity, helper.sv_funcs).is_some())
            {
                brain.next_cmd.move_.z = 1.;
                brain.next_cmd.buttons |= MoveAction::Jump as u32;
            }

            (Status::Success, 0.)
        }
        BotAction::IsFenceHop => match brain.path.front() {
            Some(point)
                if point.z > brain.abs_origin.z + 10. || bot.m_vecAbsVelocity == Vector3::ZERO =>
            {
                (Status::Success, 0.)
            }
            Some(_) => (Status::Failure, 0.),
            None => (Status::Failure, 0.),
        },
        BotAction::TryMountFence(mut frames) => {
            frames += 1;

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
            if frames % 12 < 5
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
                (Status::Failure, 0.)
            }
        }
        BotAction::IsCrawling => match (brain.path.front(), brain.navmesh.try_read()) {
            (Some(point), Some(navmesh))
                if let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh
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
        BotAction::Crawl => {
            brain.next_cmd.buttons |= MoveAction::Duck as u32;

            (Status::Success, 0.)
        }
        BotAction::IsWallRun => {
            // anything below 1 distance off the ground isn't wallrunnable
            // we find if the next point is wallrunable that means that this mean isn't some random spot where we are passing near the wall
            match (brain.path.front(), brain.navmesh.try_read()) {
                (Some(point), Some(ref navmesh))
                    if point.as_distance() > 1
                        && let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh
                        && let Some(wall_point) = get_neighbors_h(*point.as_ref(), octtree)
                            .find_map(|(point, is_empty)| is_empty.not().then_some(point))
                        && get_neighbors_h(*point.as_ref(), octtree)
                            .filter(|(_, is_empty)| *is_empty)
                            .flat_map(|(point, _)| get_neighbors_h(point, octtree).zip([point; 4]))
                            .filter_map(|((next_wall_point, is_empty), next_point)| {
                                is_empty.not().then_some((next_wall_point, next_point))
                            })
                            .find_map(|(next_wall_point, next_point)| {
                                ((wall_point.0.x == next_wall_point.0.x
                                    && (wall_point.0.y as i32 - next_wall_point.0.y as i32).abs()
                                        == 1)
                                    || (wall_point.0.y == next_wall_point.0.y
                                        && (wall_point.0.x as i32 - next_wall_point.0.x as i32)
                                            .abs()
                                            == 1))
                                    .then_some(next_point)
                            })
                            .is_some() =>
                {
                    brain.next_wall_point = unsafe {
                        let mut result = MaybeUninit::<TraceResults>::zeroed();
                        (helper.sv_funcs.util_trace_line)(
                            point.as_ref(),
                            &tuvec_to_vector3(navmesh.cell_size, wall_point),
                            i8::MAX,
                            i8::MAX,
                            i32::MAX,
                            i32::MAX,
                            i32::MAX,
                            result.as_mut_ptr(),
                        );
                        Some(result.assume_init().end_pos)
                    };

                    (Status::Success, 0.)
                }
                _ => (Status::Failure, 0.),
            }
        }
        BotAction::WallRun => (Status::Success, 0.),
        BotAction::FinishMove => '_move: {
            let Some(_) = brain.path.front() else {
                break '_move (Status::Failure, 0.);
            };

            let hitbox: Capsule = Capsule::new_z(50., 25.)
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

            // look 7 points ahead for when a bot overshoots points
            if brain
                .path
                .iter()
                .take(7)
                .map(AsRef::as_ref)
                .any(is_in_hitbox)
            {
                brain.path.pop_front();
            }

            brain.next_wall_point = None;
            brain.needs_new_path = brain.path.len() < 3;

            (Status::Success, 0.)
        }
        BotAction::IsDead => {
            if brain.last_alive_state {
                (Status::Failure, 0.)
            } else {
                (Status::Success, 0.)
            }
        }

        BotAction::DeadState => {
            brain.needs_new_path = true;
            brain.path.clear();
            (Status::Success, 0.)
        }
    });

    if bt.is_finished() {
        bt.reset_bt();
    }

    bt.blackboard().next_cmd
}
