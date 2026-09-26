use std::cell::UnsafeCell;

use crate::{
    bindings::{Action, CUserCmd, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::iterate_c_array_sized,
};
use rrplug::{
    bindings::{class_types::client::SignonState, server::cbaseentity::CBaseEntity},
    high::{UnsafeHandle, vector::Vector3},
    prelude::EngineToken,
};
use shared::utils::nudge_type;

use super::{BOT_DATA_MAP, SHARED_BOT_DATA, SIMULATE_TYPE_CONVAR, cmds_helper::CUserCmdHelper};

static LAST_CMD: UnsafeHandle<UnsafeCell<Option<CUserCmd>>> =
    unsafe { UnsafeHandle::new(UnsafeCell::new(None)) };

pub fn replace_cmd() -> Option<&'static CUserCmd> {
    unsafe { LAST_CMD.get().get().as_ref()?.as_ref() }
}

pub fn run_bots_cmds(paused: bool) {
    let sim_type = SIMULATE_TYPE_CONVAR.wait().get_value_i32();
    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let globals =
        unsafe { engine_functions.globals.as_mut() }.expect("globals were null for some reason");

    let token = unsafe { EngineToken::new_unchecked() };
    let mut bot_local_data = BOT_DATA_MAP.get(token).borrow_mut();
    let mut bot_shared_data = SHARED_BOT_DATA.get(token).borrow_mut();

    crate::PLUGIN
        .wait()
        .bots
        .external_simulations
        .simulations
        .read()
        .iter()
        .filter_map(|(_, sim)| sim.pre_simulate)
        .for_each(|pre_simulate| pre_simulate(paused));

    for (player, edict) in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| client.m_nSignonState == SignonState::FULL)
            .filter(|(_, client)| client.m_bFakePlayer)
            .filter_map(|(i, client)| {
                let bot_player = player_by_index((i + 1) as i32).as_mut()?;
                let handle = client.m_nHandle as usize - 1; // eh

                (server_functions.calc_origin)(
                    nudge_type::<&CBaseEntity>(bot_player),
                    &std::ptr::from_ref(bot_player),
                    0,
                    0,
                );

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
            (server_functions.player_process_usercmds)(
                player,
                &cmd,
                1, // was amount
                1, // was amount
                0, // was amount as u32, seams like it was causing the dropped packets spam but also it was stoping the bots from going faster?
                paused as i8,
            );

            (server_functions.simulate_player)(player)
        }
        unsafe {
            *LAST_CMD.get().get() = None;
        };
    }
}
