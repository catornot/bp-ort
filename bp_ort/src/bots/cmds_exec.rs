use std::cell::UnsafeCell;

use crate::{
    bindings::{Action, CUserCmd, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::iterate_c_array_sized,
};
use rrplug::{
    bindings::class_types::client::SignonState,
    high::{vector::Vector3, UnsafeHandle},
    prelude::EngineToken,
};

use super::{cmds_helper::CUserCmdHelper, BOT_DATA_MAP, SHARED_BOT_DATA, SIMULATE_TYPE_CONVAR};

static LAST_CMD: UnsafeHandle<UnsafeCell<Option<CUserCmd>>> =
    unsafe { UnsafeHandle::new(UnsafeCell::new(None)) };

pub fn replace_cmd() -> Option<&'static CUserCmd> {
    unsafe { LAST_CMD.get().get().as_ref()?.as_ref() }
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

    let token = unsafe { EngineToken::new_unchecked() };
    let mut bot_local_data = BOT_DATA_MAP.get(token).borrow_mut();
    let mut bot_shared_data = SHARED_BOT_DATA.get(token).borrow_mut();

    for (player, edict) in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| client.m_nSignonState == SignonState::FULL)
            .filter(|(_, client)| client.m_bFakePlayer)
            .filter_map(|(i, client)| {
                let bot_player = player_by_index((i + 1) as i32).as_mut()?;
                let handle = client.m_nHandle as usize;

                (server_functions.calc_origin)(bot_player, &std::ptr::from_ref(bot_player), 0, 0);

                Some((bot_player, handle))
            })
    } {
        let mut cmd = {
            let Some(local_data) = bot_local_data.get_mut(edict) else {
                log::warn!("bot {edict} without valid local data entry");
                continue;
            };
            local_data.edict = edict as u16;

            let helper = CUserCmdHelper::new(
                globals,
                Vector3::ZERO,
                0,
                server_functions,
                engine_functions,
            );

            super::cmds::get_cmd(
                player,
                &helper,
                local_data.sim_type.unwrap_or(sim_type),
                local_data,
                &mut bot_shared_data,
            )
            .unwrap_or_else(|| CUserCmd::new_empty(&helper))
        };

        cmd.frame_time = globals.frameTime;
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
            if !player.m_animActive || cmd.buttons & Action::WeaponDiscard as u32 == 0 {
                let frametime = globals.frameTime;
                let cur_time = globals.curTime;

                player.pl.fixangle = 0;
                set_base_time(player, cur_time);

                *std::ptr::from_mut(&mut player.m_pCurrentCommand).cast() = &cmd;

                let move_helper = move_helper()
                    .cast_mut()
                    .as_mut()
                    .expect("move_helper should not be null");

                move_helper.host = player;

                player_run_command(player, &mut cmd, move_helper);
                player.m_latestCommandRun = cmd.command_number as i32;
                // (server_functions.set_last_cmd)(
                //     (player as *const _ as *const CUserCmd)
                //         .offset(0x20a0)
                //         .cast_mut(),
                //     &mut cmd,
                // );

                move_helper.host = std::ptr::null_mut();
                globals.frameTime = frametime;
                globals.curTime = cur_time;

                // is this needed?
                // looks like it's not
                // (server_functions.simulate_player)(player);
            } else {
                run_null_command(player);
            }
            // *player.angles.get_inner_mut() = cmd.world_view_angles // this is not really great -> bad aim

            // this is still a bit jitary :(
            if globals.frameCount % 10 == 0 {
                // HACK: so setting origin forces the game to check touching so kind of fixes that but doesn't work for exiting triggers maybe?
                (server_functions.calc_origin)(player, &std::ptr::from_ref(player), 0, 0);
                (server_functions.set_origin_hack_do_not_use)(player, &player.m_vecAbsOrigin);
            }

            // *std::ptr::from_mut(player).byte_offset(0x618).cast::<u32>() = 0; // m_collectedInvalidateFlags
            // *std::ptr::from_mut(player).byte_offset(0x61c).cast::<bool>() = false; // m_collectingInvalidateFlags
            // (server_functions.perform_collision_check)(player, 1);
            // (server_functions.another_perform_collision_check)(player, std::ptr::null());
        }
        unsafe {
            *LAST_CMD.get().get() = None;
        };
    }
}
