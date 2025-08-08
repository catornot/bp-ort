use bonsai_bt::{Action, Behavior::While, Event, Float, Sequence, Status, UpdateArgs, BT, RUNNING};
use itertools::Itertools;
use oktree::prelude::*;
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
};

use crate::{
    loader::{map_to_i32, map_to_u32, Navmesh, NavmeshStatus},
    pathfinding::find_path,
    ENGINE_INTERFACES,
};

static BEHAVIOR: LazyLock<RwLock<HashMap<u16, BT<BotAction, BotBrain>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

struct BotBrain {
    navmesh: Arc<RwLock<Navmesh>>,
    current_target: Option<i32>,
    path: Vec<Vector3>,
    next_cmd: CUserCmd,
    last_alive_state: bool,
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
            path: Vec::new(),
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
        dt: helper.globals.tickInterval as f64,
    });

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

            let NavmeshStatus::Loaded(navmesh_tree) = &navmesh.navmesh else {
                break 'path (Status::Failure, 0.);
            };

            let mut v = Vector3::ZERO;
            let start = vector3_to_tuvec(navmesh.cell_size, unsafe { *player.get_origin(&mut v) });
            let end = vector3_to_tuvec(navmesh.cell_size, unsafe {
                *other_player.get_origin(&mut v)
            });

            log::info!("pathfinding from {start:?} to {end:?}");

            brain.path = find_path(navmesh_tree, start, end)
                .into_iter()
                .flatten()
                .map(|point| tuvec_to_vector3(navmesh.cell_size, point))
                .collect();

            (Status::Success, 0.)
        }
        BotAction::RenderPath => 'render: {
            log::info!("render");
            if brain.path.is_empty() {
                break 'render (Status::Failure, 0.);
            }
            log::info!("render2");

            let debug = ENGINE_INTERFACES.wait().debug_overlay;
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

    if bt.is_finished() {
        bt.reset_bt();
    }

    bt.blackboard().next_cmd
}

fn vector3_to_tuvec(cell_size: f32, origin: Vector3) -> TUVec3u32 {
    let scaled = origin / Vector3::new(cell_size, cell_size, cell_size);

    TUVec3u32::new(
        map_to_u32(scaled.x as i32),
        map_to_u32(scaled.y as i32),
        map_to_u32(scaled.z as i32),
    )
}

fn tuvec_to_vector3(cell_size: f32, point: TUVec3u32) -> Vector3 {
    Vector3::new(cell_size, cell_size, cell_size)
        * Vector3::new(
            map_to_i32(point.0.x) as f32,
            map_to_i32(point.0.y) as f32,
            map_to_i32(point.0.z) as f32,
        )
}
