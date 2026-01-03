use bevy_math::UVec3;
use bonsai_bt::{
    Action,
    Behavior::{AlwaysSucceed, If, Select, WhenAny},
    Event, Sequence, Status, UpdateArgs, BT, RUNNING,
};
use itertools::Itertools;
use oktree::prelude::*;
use parking_lot::RwLock;
use rrplug::{
    bindings::class_types::{cbaseentity::CBaseEntity, client::CClient, cplayer::CPlayer},
    prelude::*,
};
use shared::{
    bindings::{CUserCmd, Contents, TraceCollisionGroup, SERVER_FUNCTIONS},
    cmds_helper::CUserCmdHelper,
    utils::{get_player_index, is_alive, lookup_ent, nudge_type, trace_ray},
};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    sync::{Arc, LazyLock},
};

use crate::{
    async_pathfinding::{GoalFloat, PathReceiver},
    loader::{Navmesh, NavmeshStatus, Octree32},
    movement::{run_movement, Movement, MovementAction},
    nav_points::{tuvec_to_vector3, vector3_to_tuvec, NavPoint},
    pathfinding::{find_path, AreaCost, Goal},
    targeting::{run_targeting, Target, Targeting, TargetingAction},
};

static BEHAVIOR: LazyLock<RwLock<HashMap<u16, BT<BotAction, BotBrain>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct BotBrain {
    pub navmesh: Arc<RwLock<Navmesh>>,
    pub path_receiver: Option<PathReceiver>,
    pub path_next_request: f32,
    pub path: VecDeque<NavPoint>,
    pub origin: Vector3,
    pub angles: Vector3,
    pub abs_origin: Vector3,
    pub next_cmd: CUserCmd,
    pub last_alive_state: bool,
    pub looked_at_death_record: bool,
    pub needs_new_path: bool,

    /// movement
    pub m: Movement,
    pub t: Targeting,
}

#[derive(Debug, Clone)]
pub enum BotAction {
    RenderPath,
    FindPath,
    CheckNavmesh,
    IsDead,
    DeadState,
    Movement(MovementAction),
    Targeting(TargetingAction),
}

pub fn drop_behaviors() {
    BEHAVIOR.write().clear();
}

pub extern "C" fn init_bot(edict: u16, client: &CClient) {
    let target_moving = Sequence(vec![
        Select(vec![
            // because we can't stop moving if we are wallrunning
            Action(MovementAction::IsWallRun.into()),
            Action(MovementAction::CanMove.into()),
        ]),
        Action(MovementAction::CheckReachability.into()),
        AlwaysSucceed(Box::new(Select(vec![
            Sequence(vec![
                Action(MovementAction::IsJump.into()),
                Action(MovementAction::Jump.into()),
            ]),
            Sequence(vec![
                Action(MovementAction::IsCrawling.into()),
                Action(MovementAction::Crawl.into()),
            ]),
            Sequence(vec![
                Action(MovementAction::IsFenceHop.into()),
                Action(MovementAction::TryMountFence.into()),
            ]),
            Sequence(vec![
                Action(MovementAction::IsGoingDown.into()),
                Action(MovementAction::GoDownBetter.into()),
            ]),
        ]))),
        AlwaysSucceed(Box::new(Action(MovementAction::Move.into()))),
        Action(MovementAction::FinishMove.into()),
    ]);

    let targetting = Sequence(vec![
        AlwaysSucceed(Box::new(Select(vec![
            Action(TargetingAction::Melee.into()),
            Action(TargetingAction::Shoot.into()),
        ]))),
        Action(TargetingAction::TargetSwitching.into()),
    ]);

    let target_tracking = Sequence(vec![
        Action(TargetingAction::FindTarget.into()),
        Action(BotAction::RenderPath),
        Action(MovementAction::StartMoving.into()),
        WhenAny(vec![
            Action(BotAction::FindPath),
            Sequence(vec![targetting, AlwaysSucceed(Box::new(target_moving))]),
        ]),
    ]);

    let routine = Sequence(vec![
        Action(BotAction::CheckNavmesh),
        If(
            Box::new(Action(BotAction::IsDead)),
            Box::new(Action(BotAction::DeadState)),
            Box::new(target_tracking),
        ),
    ]);

    log::info!(
        "init bot {} with {edict}",
        shared::utils::get_c_char_array_lossy(&client.m_szServerName)
    );
    BEHAVIOR.write().entry(edict).insert_entry(BT::new(
        routine,
        BotBrain {
            navmesh: Arc::clone(&crate::PLUGIN.wait().navmesh),
            next_cmd: CUserCmd::init_default(SERVER_FUNCTIONS.wait()),
            last_alive_state: false,
            looked_at_death_record: true,
            path_receiver: None,
            path: VecDeque::new(),
            path_next_request: 0.,
            origin: Vector3::ZERO,
            angles: Vector3::ZERO,
            abs_origin: Vector3::ZERO,
            needs_new_path: true,

            t: Targeting {
                current_target: Target::None,
                last_target: Target::None,
                reacts_at: 0.,
                spread: Vec::new(),
                spread_rigth: true,
                hates: BTreeMap::default(),
            },

            m: Movement {
                can_move: true,
                next_wall_point: None,
                next_point_override: None,
                view_lock: false,
                clamped_view: Some(0.),
                jump_tick: 0,
                vault_tick: 0,
                down_tick: 0,
                area_cost: AreaCost::default(),
                last_path_points: Vec::new(),
                last_point_reached_delta: 0.,
            },
        },
    ));
}

pub extern "C" fn wallpathfining_bots(helper: &CUserCmdHelper, bot: &mut CPlayer) -> CUserCmd {
    let mut behavior_static = BEHAVIOR.write();
    let Some(bt) = behavior_static.get_mut(&(get_player_index(bot) as u16)) else {
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

    if is_alive(bot) != bt.blackboard().last_alive_state {
        bt.reset_bt();
        bt.blackboard_mut().last_alive_state = is_alive(bot);
    }

    let e = Event::from(UpdateArgs { dt: 0. });

    bt.tick(&e, &mut |args, brain| match args.action {
        BotAction::CheckNavmesh => {
            if matches!(brain.navmesh.read().navmesh, NavmeshStatus::Loaded(_)) {
                (Status::Success, 0.)
            } else {
                (Status::Failure, 0.)
            }
        }
        BotAction::FindPath => 'path: {
            let navmesh = brain.navmesh.read();
            let Some(end) = brain.t.current_target.to_goal(helper) else {
                break 'path (Status::Failure, 0.);
            };

            let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh else {
                break 'path (Status::Failure, 0.);
            };

            if !brain.needs_new_path {
                break 'path RUNNING;
            }

            // dedup this function lol
            fn distance2d(p: Vector3, v: Vector3) -> f32 {
                ((p.x - v.x).powi(2) + ((p.y - v.y).powi(2))).sqrt()
            }
            // TODO: test if this actually works lol
            if brain.path.is_empty()
                && brain.path_receiver.is_none()
                && let Some(distance) = brain
                    .t
                    .current_target
                    .to_position(helper)
                    .map(|pos| distance2d(pos, brain.abs_origin))
                && distance > 1000.
                && let Some(pos) = match end {
                    GoalFloat::Point(pos) | GoalFloat::ClosestToPoint(pos) => Some(pos),
                    _ => None,
                }
            {
                brain.path.extend(
                    find_path::<1000>(
                        octtree,
                        brain.m.area_cost.clone(),
                        find_closest_navpoint(
                            brain.origin,
                            bot,
                            navmesh.cell_size,
                            octtree,
                            helper,
                        )
                        .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, brain.origin)),
                        Goal::ClosestToPoint(
                            find_closest_navpoint(pos, bot, navmesh.cell_size, octtree, helper)
                                .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, pos)),
                        ),
                        navmesh.cell_size,
                    )
                    .into_iter()
                    .flat_map(|vec| vec.into_iter()),
                );
            }

            let start = brain
                .path
                .back()
                .map(AsRef::as_ref)
                .copied()
                .unwrap_or(brain.origin);

            if let None = brain.path_receiver.as_ref()
                && brain.path_next_request < helper.globals.curTime
            {
                brain.path_receiver = crate::PLUGIN.wait().job_market.find_path(
                    tuvec_to_vector3(
                        navmesh.cell_size,
                        find_closest_navpoint(start, bot, navmesh.cell_size, octtree, helper)
                            .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, start)),
                    ),
                    match end {
                        GoalFloat::Point(end) => GoalFloat::Point(tuvec_to_vector3(
                            navmesh.cell_size,
                            find_closest_navpoint(end, bot, navmesh.cell_size, octtree, helper)
                                .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, end)),
                        )),
                        GoalFloat::ClosestToPoint(end) => {
                            GoalFloat::ClosestToPoint(tuvec_to_vector3(
                                navmesh.cell_size,
                                find_closest_navpoint(end, bot, navmesh.cell_size, octtree, helper)
                                    .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, end)),
                            ))
                        }
                        GoalFloat::Distance(distance) => GoalFloat::Distance(distance),
                        GoalFloat::Area(pos, radius) => GoalFloat::Area(pos, radius),
                    },
                    brain.m.area_cost.clone(),
                );
            }

            let status = if let Some(path_receiver) = brain.path_receiver.as_ref() {
                match path_receiver.try_recv() {
                    Ok(Some(path)) => {
                        brain.m.last_path_points.clear();
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
            if status.0 == Status::Failure {
                RUNNING
            } else {
                status
            }
        }
        BotAction::RenderPath => {
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

        BotAction::Movement(movement) => run_movement(movement, brain, bot, helper),

        BotAction::Targeting(targeting) => run_targeting(targeting, brain, bot, helper),

        BotAction::IsDead => {
            if brain.last_alive_state {
                brain.looked_at_death_record = false;
                (Status::Failure, 0.)
            } else {
                (Status::Success, 0.)
            }
        }

        BotAction::DeadState => {
            brain.needs_new_path = true;
            // this a bit dangerous since if FindPath is running in parallel it can cause lot's of tasks to get pushed to the worker threads which will overwhelm them
            brain.path_receiver = None; // clear any paths under construction
            brain.path.clear();

            if let Some(player) = lookup_ent(bot.m_lastDeathInfo.m_hAttacker, helper.sv_funcs)
                .and_then::<&CPlayer, _>(|ent| ent.dynamic_cast())
                && !brain.looked_at_death_record
            {
                *brain.t.hates.entry(get_player_index(player)).or_default() += 1;
                brain.looked_at_death_record = true;
            }

            (Status::Success, 0.)
        }
    });

    if bt.is_finished() {
        bt.reset_bt();
    }

    bt.blackboard().next_cmd
}

pub extern "C" fn test_closest_navpoint(helper: &CUserCmdHelper, bot: &mut CPlayer) -> CUserCmd {
    let mut behavior_static = BEHAVIOR.write();
    let Some(bt) = behavior_static.get_mut(&(get_player_index(bot) as u16)) else {
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

    let navmesh = bt.blackboard().navmesh.read();
    let NavmeshStatus::Loaded(octtree) = &navmesh.navmesh else {
        return CUserCmd::new_empty(helper);
    };

    if let Some(point) = find_closest_navpoint(
        bt.blackboard().origin,
        bot,
        navmesh.cell_size,
        octtree,
        helper,
    ) {
        let debug = crate::ENGINE_INTERFACES.wait().debug_overlay;
        unsafe {
            debug.AddLineOverlay(
                &bt.blackboard().origin,
                &tuvec_to_vector3(navmesh.cell_size, point),
                200,
                0,
                0,
                true,
                0.1,
            )
        }
    } else {
        log::warn!(
            "no point found {}",
            unsafe { std::ffi::CStr::from_ptr(bot.m_szNetname.as_ptr()) }.to_string_lossy()
        );
    }

    CUserCmd::new_empty(helper)
}

/// aka a empty one
/// TODO: cleanup this shit
/// holy shit this is so bad
pub fn find_closest_navpoint(
    position: Vector3,
    ent: &CBaseEntity,
    cell_size: f32,
    navmesh: &Octree32,
    helper: &CUserCmdHelper,
) -> Option<TUVec3u32> {
    let mut searched = BTreeSet::default();
    fn search_neighboors<'a>(
        start: Vector3,
        ent: Option<&CBaseEntity>,
        point: TUVec3u32,
        cell_size: f32,
        navmesh: &'a Octree32,
        helper: &'a CUserCmdHelper,
        searched: &'a mut BTreeSet<TUVec3u32>,
    ) -> Vec<Result<TUVec3u32, TUVec3u32>> {
        [
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
            [-1, 0, 0],
            [0, -1, 0],
            [0, 0, -1],
        ]
        .into_iter()
        .flat_map(|offset| {
            Some(TUVec3u32::new(
                point.0.x.checked_add_signed(offset[0])?,
                point.0.y.checked_add_signed(offset[1])?,
                point.0.z.checked_add_signed(offset[2])?,
            ))
        })
        .flat_map(|neighboor| {
            if searched.contains(&neighboor) {
                return None;
            }
            _ = searched.insert(neighboor);

            if navmesh.get(&neighboor.0).is_some() {
                return Some(Err(neighboor));
            }

            (trace_ray(
                start,
                tuvec_to_vector3(cell_size, neighboor),
                ent,
                TraceCollisionGroup::None,
                Contents::SOLID
                    | Contents::MOVEABLE
                    | Contents::WINDOW
                    | Contents::MONSTER
                    | Contents::GRATE
                    | Contents::PLAYER_CLIP,
                helper.sv_funcs,
                helper.engine_funcs,
            )
            .fraction
                == 1.)
                .then_some(Ok(neighboor))
        })
        .collect::<Vec<Result<_, _>>>()
    }

    let initial_point = vector3_to_tuvec(cell_size, position);
    let mut searches = vec![Err(initial_point)];
    while let Some(search) = searches
        .iter()
        .copied()
        .find(|r| r.is_ok())
        .or_else(|| searches.pop())
        && searched.len() <= 32
    {
        match search {
            Ok(point) => {
                // TODO: check for wallrun
                return Some(point).map(|point| snap_to_ground(navmesh, point).unwrap_or(point));
            }
            Err(bad_point) => searches.extend_from_slice(&search_neighboors(
                position,
                Some(ent),
                bad_point,
                cell_size,
                navmesh,
                helper,
                &mut searched,
            )),
        }
        searches.sort_by(|this, other| {
            let this_unwrapped = match this {
                Ok(pos) | Err(pos) => pos,
            };
            let other_unwrapped = match other {
                Ok(pos) | Err(pos) => pos,
            };
            (UVec3::new(initial_point.0.x, initial_point.0.y, initial_point.0.z)
                .as_vec3()
                .distance(
                    UVec3::new(this_unwrapped.0.x, this_unwrapped.0.y, this_unwrapped.0.z)
                        .as_vec3(),
                ))
            .total_cmp(
                &UVec3::new(initial_point.0.x, initial_point.0.y, initial_point.0.z)
                    .as_vec3()
                    .distance(
                        UVec3::new(
                            other_unwrapped.0.x,
                            other_unwrapped.0.y,
                            other_unwrapped.0.z,
                        )
                        .as_vec3(),
                    ),
            )
            .reverse()
        });
    }
    None
}

fn snap_to_ground(octtree: &Octree32, point: TUVec3u32) -> Option<TUVec3u32> {
    (point.0.z.saturating_sub(1000)..point.0.z)
        .rev()
        .find_map(|z| {
            octtree
                .get(&TUVec3::new(point.0.x, point.0.y, z))
                .is_some()
                .then_some(TUVec3u32::new(point.0.x, point.0.y, z.checked_add(1)?))
        })
}
