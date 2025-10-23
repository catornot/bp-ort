use bevy_math::prelude::*;
use bonsai_bt::{
    Action,
    Behavior::{AlwaysSucceed, WhenAny},
    Event, Sequence, Status, UpdateArgs, BT, RUNNING,
};
use itertools::Itertools;
use parking_lot::RwLock;
use parry3d::shape::Capsule;
use parry3d::{self, query::RayCast};
use rrplug::{
    bindings::class_types::{cbaseentity::CBaseEntity, client::CClient, cplayer::CPlayer},
    prelude::*,
};
use shared::{
    bindings::{Action as MoveAction, CUserCmd, SERVER_FUNCTIONS},
    cmds_helper::CUserCmdHelper,
    utils::{lookup_ent, nudge_type},
};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, LazyLock},
};

use crate::{
    async_pathfinding::PathReceiver,
    loader::{Navmesh, NavmeshStatus},
};

static BEHAVIOR: LazyLock<RwLock<HashMap<u16, BT<BotAction, BotBrain>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

struct BotBrain {
    navmesh: Arc<RwLock<Navmesh>>,
    current_target: Option<i32>,
    path_receiver: Option<PathReceiver>,
    path_next_request: f32,
    path: VecDeque<Vector3>,
    origin: Vector3,
    abs_origin: Vector3,
    next_cmd: CUserCmd,
    last_alive_state: bool,
    needs_new_path: bool,
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
    FinishMove,
}

pub fn drop_behaviors() {
    BEHAVIOR.write().clear();
}

pub extern "C" fn init_bot(edict: u16, client: &CClient) {
    let target_moving = Sequence(vec![
        Action(BotAction::CanMove),
        Action(BotAction::Move),
        AlwaysSucceed(Box::new(Sequence(vec![
            Action(BotAction::IsJump),
            Action(BotAction::Jump(0)),
        ]))),
        Action(BotAction::FinishMove),
    ]);

    let dead_state = Sequence(vec![
        Action(BotAction::IsDead),
        Action(BotAction::DeadState),
    ]);

    let target_moving = Sequence(vec![
        Action(BotAction::CanMove),
        Action(BotAction::Move),
        AlwaysSucceed(Box::new(Sequence(vec![
            Action(BotAction::IsJump),
            Action(BotAction::Jump(0)),
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
            abs_origin: Vector3::ZERO,
            needs_new_path: true,
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
    bt.blackboard_mut().abs_origin = bot.m_vecAbsOrigin;

    let is_alive = unsafe { (helper.sv_funcs.is_alive)(nudge_type::<&CBaseEntity>(bot)) } == 1;
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
            let start = brain.path.back().copied().unwrap_or(brain.origin);
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
                    debug.AddLineOverlay(&p1, &p2, 0, 255, 0, true, 0.5)
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
                        brain.path.front().unwrap_or(&Vector3::ZERO),
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
            let Some(target) = brain.path.front().copied() else {
                break '_move (Status::Failure, 0.);
            };

            let mut forward_vector = Vector3::ZERO;
            unsafe {
                bot.get_forward_vector(&mut forward_vector, std::ptr::null(), std::ptr::null())
            };
            let forward_vector = Vec2::new(forward_vector.x, forward_vector.y);

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
            Some(point) if point.z > brain.origin.z => (Status::Success, 0.),
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
        BotAction::FinishMove => '_move: {
            let Some(target) = brain.path.front() else {
                break '_move (Status::Failure, 0.);
            };

            if Capsule::new_z(50., 25.)
                .transform_by(&[brain.origin.x, brain.origin.y, brain.origin.z].into())
                .intersects_local_ray(
                    &parry3d::query::Ray::new(
                        [target.x, target.y, target.z].into(),
                        [0., 0., 1.].into(),
                    ),
                    0.01,
                )
            {
                brain.path.pop_front();
            }

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
