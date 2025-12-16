use bevy_math::prelude::*;
use bonsai_bt::{
    Action,
    Behavior::{AlwaysSucceed, If, Select, WhenAny},
    Event, Sequence, Status, UpdateArgs, BT, RUNNING,
};
use itertools::Itertools;
use parking_lot::RwLock;
use rrplug::{
    bindings::class_types::{client::CClient, cplayer::CPlayer},
    prelude::*,
};
use shared::{
    bindings::{CUserCmd, SERVER_FUNCTIONS},
    cmds_helper::CUserCmdHelper,
    utils::{get_player_index, lookup_ent, nudge_type},
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Arc, LazyLock},
};

use crate::{
    async_pathfinding::PathReceiver,
    loader::{Navmesh, NavmeshStatus},
    movement::{run_movement, Movement, MovementAction},
    nav_points::NavPoint,
    pathfinding::AreaCost,
};

static BEHAVIOR: LazyLock<RwLock<HashMap<u16, BT<BotAction, BotBrain>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

pub struct BotBrain {
    pub navmesh: Arc<RwLock<Navmesh>>,
    pub current_target: Option<i32>,
    pub path_receiver: Option<PathReceiver>,
    pub path_next_request: f32,
    pub path: VecDeque<NavPoint>,
    pub origin: Vector3,
    pub angles: Vector3,
    pub abs_origin: Vector3,
    pub next_cmd: CUserCmd,
    pub last_alive_state: bool,
    pub needs_new_path: bool,

    // targeting
    pub hates: BTreeMap<usize, u32>,

    /// movement
    pub m: Movement,
}

#[derive(Debug, Clone)]
pub enum BotAction {
    RenderPath,
    FindTarget,
    FindPath,
    CheckNavmesh,
    IsDead,
    DeadState,
    Movement(MovementAction),
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

    let target_tracking = Sequence(vec![
        Action(BotAction::FindTarget),
        Action(BotAction::RenderPath),
        WhenAny(vec![Action(BotAction::FindPath), target_moving]),
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
            current_target: None,
            next_cmd: CUserCmd::init_default(SERVER_FUNCTIONS.wait()),
            last_alive_state: false,
            path_receiver: None,
            path: VecDeque::new(),
            path_next_request: 0.,
            origin: Vector3::ZERO,
            angles: Vector3::ZERO,
            abs_origin: Vector3::ZERO,
            needs_new_path: true,

            hates: BTreeMap::new(),

            m: Movement {
                next_wall_point: None,
                next_point_override: None,
                view_lock: false,
                clamped_view: 0.,
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

    let is_alive = bot.m_lifeState == 0;
    if is_alive != bt.blackboard().last_alive_state {
        bt.reset_bt();
        bt.blackboard_mut().last_alive_state = is_alive;
    }

    let e = Event::from(UpdateArgs { dt: 0. });

    bt.tick(&e, &mut |args, brain| match args.action {
        BotAction::FindTarget => 'target: {
            let mut v = Vector3::ZERO;
            let base = Vec3::new(brain.origin.x, brain.origin.y, brain.origin.z);
            let Some(current_target) = (0..helper.globals.maxPlayers)
                .flat_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_ref() })
                .filter(|other| get_player_index(other) != get_player_index(bot))
                .map(|other| {
                    (
                        unsafe { *other.get_origin(&mut v) },
                        get_player_index(other),
                    )
                })
                .map(|(Vector3 { x, y, z }, index)| (Vec3::new(x, y, z), index))
                .reduce(|left, rigth| {
                    if (left.0.distance(base) as u32)
                        .saturating_sub(brain.hates.get(&left.1).copied().unwrap_or_default() * 50)
                        < (rigth.0.distance(base) as u32).saturating_sub(
                            brain.hates.get(&rigth.1).copied().unwrap_or_default() * 50,
                        )
                    {
                        left
                    } else {
                        rigth
                    }
                })
                .map(|(_, index)| index)
            else {
                break 'target (Status::Failure, 0.);
            };
            brain.current_target = Some(current_target as i32 + 1);

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
                brain.path_receiver = crate::PLUGIN.wait().job_market.find_path(
                    start,
                    end,
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

        BotAction::Movement(movement) => run_movement(movement, brain, bot, helper),

        BotAction::IsDead => {
            if brain.last_alive_state {
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
            {
                *brain.hates.entry(get_player_index(player)).or_default() += 1;
            }

            (Status::Success, 0.)
        }
    });

    if bt.is_finished() {
        bt.reset_bt();
    }

    bt.blackboard().next_cmd
}
