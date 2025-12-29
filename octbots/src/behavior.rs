use bonsai_bt::{
    Action,
    Behavior::{AlwaysSucceed, If, Select, WhenAny},
    Event, Sequence, Status, UpdateArgs, BT, RUNNING,
};
use itertools::Itertools;
use oktree::prelude::TUVec3u32;
use parking_lot::RwLock;
use rrplug::{
    bindings::class_types::{client::CClient, cplayer::CPlayer},
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
    async_pathfinding::PathReceiver,
    loader::{Navmesh, NavmeshStatus, Octree32},
    movement::{run_movement, Movement, MovementAction},
    nav_points::{tuvec_to_vector3, vector3_to_tuvec, NavPoint},
    pathfinding::AreaCost,
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
        Action(MovementAction::CanMove.into()),
        Action(MovementAction::CheckReachability.into()),
        AlwaysSucceed(Box::new(Sequence(vec![
            Action(MovementAction::IsWallRun.into()),
            Action(MovementAction::WallRun.into()),
        ]))),
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
        Action(MovementAction::Move.into()),
        Action(MovementAction::FinishMove.into()),
    ]);

    let targetting = Sequence(vec![
        Action(TargetingAction::Shoot.into()),
        Action(TargetingAction::TargetSwitching.into()),
    ]);

    let target_tracking = Sequence(vec![
        Action(TargetingAction::FindTarget.into()),
        Action(BotAction::RenderPath),
        Action(MovementAction::StartMoving.into()),
        WhenAny(vec![
            Action(BotAction::FindPath),
            Sequence(vec![targetting, target_moving]),
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
            let Some(end) = brain.t.current_target.to_position(helper) else {
                break 'path (Status::Failure, 0.);
            };

            let NavmeshStatus::Loaded(octree) = &navmesh.navmesh else {
                break 'path (Status::Failure, 0.);
            };

            if !brain.needs_new_path {
                break 'path RUNNING;
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
                        find_closest_navpoint(start, navmesh.cell_size, octree, helper)
                            .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, start)),
                    ),
                    tuvec_to_vector3(
                        navmesh.cell_size,
                        find_closest_navpoint(end, navmesh.cell_size, octree, helper)
                            .unwrap_or_else(|| vector3_to_tuvec(navmesh.cell_size, end)),
                    ),
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

/// aka a empty one
pub fn find_closest_navpoint(
    position: Vector3,
    cell_size: f32,
    navmesh: &Octree32,
    helper: &CUserCmdHelper,
) -> Option<TUVec3u32> {
    let mut searched = BTreeSet::default();
    fn search_neighboors(
        start: Vector3,
        point: TUVec3u32,
        cell_size: f32,
        navmesh: &Octree32,
        helper: &CUserCmdHelper,
        searched: &mut BTreeSet<TUVec3u32>,
    ) -> Option<TUVec3u32> {
        if searched.len() > 32 {
            return None;
        }

        let result = [
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
                None,
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
                == 1.0)
                .then_some(Ok(neighboor))
        })
        .collect::<Vec<Result<_, _>>>();

        result.iter().find_map(|r| r.ok()).or_else(|| {
            result
                .into_iter()
                .flat_map(|r| r.err())
                .find_map(|pos| search_neighboors(start, pos, cell_size, navmesh, helper, searched))
        })
    }

    search_neighboors(
        position,
        vector3_to_tuvec(cell_size, position),
        cell_size,
        navmesh,
        helper,
        &mut searched,
    )
}
