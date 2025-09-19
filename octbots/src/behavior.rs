use bonsai_bt::{Action, Event, Sequence, Status, UpdateArgs, BT, RUNNING};
use itertools::Itertools;
use parking_lot::RwLock;
use rrplug::{
    bindings::class_types::{client::CClient, cplayer::CPlayer},
    prelude::*,
};
use shared::{
    bindings::{CUserCmd, SERVER_FUNCTIONS},
    cmds_helper::CUserCmdHelper,
};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock},
    time::Duration,
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
    path: Vec<Vector3>,
    next_cmd: CUserCmd,
    last_alive_state: bool,
    min_time: Duration,
    max_time: Duration,
    total_time: Duration,
    iterations: u32,
}

#[derive(Debug, Clone)]
pub enum BotAction {
    RenderPath,
    FindTarget,
    FindPath,
    CheckNavmesh,
}

pub fn drop_behaviors() {
    BEHAVIOR.write().clear();
}

pub extern "C" fn init_bot(edict: u16, client: &CClient) {
    let routine = Sequence(vec![
        Action(BotAction::CheckNavmesh),
        Action(BotAction::FindTarget),
        Action(BotAction::FindPath),
        Action(BotAction::RenderPath),
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
            path: Vec::new(),
            path_next_request: 0.,
            min_time: Duration::ZERO,
            max_time: Duration::ZERO,
            total_time: Duration::ZERO,
            iterations: 0,
        },
    ));
}

pub extern "C" fn wallpathfining_bots(helper: &CUserCmdHelper, player: &mut CPlayer) -> CUserCmd {
    let mut behavior_static = BEHAVIOR.write();
    let Some(bt) = behavior_static.get_mut(&(player.pl.index as u16)) else {
        return CUserCmd::new_empty(helper);
    };

    bt.blackboard_mut().next_cmd = CUserCmd::new_empty(helper);

    let is_alive = unsafe { (helper.sv_funcs.is_alive)(player) } == 1;
    if is_alive != bt.blackboard().last_alive_state {
        bt.reset_bt();
        bt.blackboard_mut().last_alive_state = is_alive;
    }

    let e = Event::from(UpdateArgs {
        dt: dbg!(helper.globals.tickInterval as f64),
    });

    let start = std::time::SystemTime::now();
    bt.tick(&e, &mut |args, brain| match args.action {
        BotAction::FindTarget => 'target: {
            let Some(current_target) = (0..helper.globals.maxPlayers)
                .flat_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_ref() })
                .filter(|other| other.pl.index != player.pl.index)
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

            let mut v = Vector3::ZERO;
            let start = unsafe { *player.get_origin(&mut v) };
            let end = unsafe { *other_player.get_origin(&mut v) };

            if let None = brain.path_receiver.as_ref()
                && brain.path_next_request < helper.globals.curTime
            {
                // log::info!("pathfinding from {start:?} to {end:?}");
                brain.path_receiver = crate::PLUGIN.wait().job_market.find_path(start, end);
            }

            // let start = std::time::SystemTime::now();

            // log::info!(
            //     "done path {:?} {}",
            //     std::time::SystemTime::now()
            //         .duration_since(start)
            //         .unwrap_or_default(),
            //     brain.path.len()
            // );
            let status = if let Some(path_receiver) = brain.path_receiver.as_ref() {
                match path_receiver.try_recv() {
                    Ok(Some(path)) => {
                        brain.path = path;

                        (Status::Success, 0.)
                    }
                    Ok(None) => {
                        brain.path_next_request = helper.globals.curTime + 0.1;
                        (Status::Failure, 0.)
                    }
                    Err(crossbeam::channel::TryRecvError::Disconnected) => (Status::Failure, 0.),
                    Err(crossbeam::channel::TryRecvError::Empty) => RUNNING,
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
                break 'render (Status::Failure, 0.);
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
    });

    let dif = std::time::SystemTime::now()
        .duration_since(start)
        .unwrap_or_default();

    bt.blackboard_mut().total_time += dif;
    bt.blackboard_mut().max_time = dif.max(bt.blackboard().max_time);
    bt.blackboard_mut().min_time = dif.min(bt.blackboard().min_time);
    bt.blackboard_mut().iterations += 1;

    // log::info!("done {dif:?}");

    if bt.is_finished() {
        bt.reset_bt();
    }

    bt.blackboard().next_cmd
}

pub extern "C" fn infodump(helper: &CUserCmdHelper, player: &mut CPlayer) -> CUserCmd {
    let mut behavior_static = BEHAVIOR.write();
    let Some(bt) = behavior_static.get_mut(&(player.pl.index as u16)) else {
        return CUserCmd::new_empty(helper);
    };

    let brain = bt.blackboard_mut();
    if brain.iterations != 0 {
        log::info!("min time {:?}", brain.min_time);
        log::info!("max time {:?}", brain.max_time);
        log::info!(
            "average time {:?}",
            brain.total_time.div_f64(brain.iterations as f64)
        );
        brain.iterations = 0;
    }

    CUserCmd::new_empty(helper)
}
