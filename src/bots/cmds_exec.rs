use itertools::Itertools;
use rand::{thread_rng, Rng};
use rrplug::{
    bindings::class_types::{client::SignonState, cplayer::CPlayer},
    high::{squirrel::call_sq_function, vector::Vector3, UnsafeHandle},
    mid::squirrel::{SQFUNCTIONS, SQVM_SERVER},
    prelude::EngineToken,
};
use std::mem::MaybeUninit;

use crate::{
    bindings::{
        Action, CBaseEntity, CGlobalVars, CTraceFilterSimple, CUserCmd, EngineFunctions, Ray,
        ServerFunctions, TraceResults, VectorAligned, ENGINE_FUNCTIONS, SERVER_FUNCTIONS,
    },
    interfaces::ENGINE_INTERFACES,
    navmesh::{Hull, RECAST_DETOUR},
    utils::{get_net_var, iterate_c_array_sized},
};

use super::{cmds_helper::CUserCmdHelper, BotData, BOT_DATA_MAP, SIMULATE_TYPE_CONVAR};

const GROUND_OFFSET: Vector3 = Vector3::new(0., 0., 20.);
const BOT_VISON_RANGE: f32 = 3000.;
const BOT_PATH_NODE_RANGE: f32 = 50.;
const BOT_PATH_RECAL_RANGE: f32 = 600.;

static mut LAST_CMD: Option<CUserCmd> = None;

pub fn replace_cmd() -> Option<&'static CUserCmd> {
    unsafe { LAST_CMD.as_ref() }
}

pub fn run_bots_cmds(_paused: bool) {
    let sim_type = SIMULATE_TYPE_CONVAR.wait().get_value_i32();
    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let run_null_command = server_functions.run_null_command;
    // let player_process_usercmds = server_functions.player_process_usercmds ;
    let set_base_time = server_functions.set_base_time;
    let player_run_command = server_functions.player_run_command;
    let move_helper = server_functions.get_move_helper;
    let globals =
        unsafe { engine_functions.globals.as_mut() }.expect("globals were null for some reason");

    let helper = CUserCmdHelper::new(
        globals,
        Vector3::ZERO,
        0,
        server_functions,
        engine_functions,
    );

    let mut bot_tasks = BOT_DATA_MAP
        .get(unsafe { EngineToken::new_unchecked() })
        .borrow_mut();

    for (mut cmd, player) in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| **client.signon == SignonState::FULL)
            .filter(|(_, client)| **client.fake_player)
            .filter_map(|(i, client)| {
                let bot_player = player_by_index((i + 1) as i32).as_mut()?;
                let edict = **client.edict as usize;

                let data = bot_tasks.as_mut().get_mut(edict)?;
                data.edict = edict as u16;
                Some((
                    super::cmds::get_cmd(
                        bot_player,
                        &helper,
                        data.sim_type.unwrap_or(sim_type),
                        data,
                    )?,
                    player_by_index((i + 1) as i32).as_mut()?,
                ))
            }) // can collect here to stop the globals from complaning about mutability
    } {
        cmd.frame_time = unsafe { globals.tick_interval.copy_inner() };
        unsafe {
            // add_user_cmd_to_player(
            //     player,
            //     &cmd,
            //     1, // was amount
            //     1, // was amount
            //     0, // was amount as u32, seams like it was causing the dropped packets spam but also it was stoping the bots from going faster?
            //     paused as i8,
            // );

            // LAST_CMD = Some(cmd);

            // bots don't trigger triggers for some reason this way

            // m_pPhysicsController may be behind the crashes in titans

            // checks for m_animActive
            // looks like it still did nothing
            if !*(player as *const _ as *const bool).offset(0xc88)
                || cmd.buttons & Action::WeaponDiscard as u32 == 0
            {
                let frametime = **globals.frametime;
                let cur_time = **globals.cur_time;

                *player.cplayer_state_fixangle.get_inner_mut() = 0;
                set_base_time(player, cur_time);

                *(player.current_command.get_inner_mut() as *mut *const _
                    as *mut *const CUserCmd) = &cmd;

                let move_helper = move_helper()
                    .cast_mut()
                    .as_mut()
                    .expect("move_helper should not be null");

                move_helper.host = player;

                player_run_command(player, &mut cmd, move_helper);
                *player.latest_command_run.get_inner_mut() = cmd.command_number;
                // (server_functions.set_last_cmd)(
                //     (player as *const _ as *const CUserCmd)
                //         .offset(0x20a0)
                //         .cast_mut(),
                //     &mut cmd,
                // );

                move_helper.host = std::ptr::null_mut();
                #[allow(invalid_reference_casting)] // tmp or not XD
                {
                    *((globals.frametime.get_inner() as *const f32).cast_mut()) = frametime;
                    *((globals.cur_time.get_inner() as *const f32).cast_mut()) = cur_time;
                }

                (server_functions.simulate_player)(player);
            } else {
                run_null_command(player);
            }
            // *player.angles.get_inner_mut() = cmd.world_view_angles // this is not really great -> bad aim
        }
        unsafe { LAST_CMD = None }
    }
}
