use std::ops::Not;

use once_cell::sync::OnceCell;
use rrplug::{
    bindings::{
        class_types::client::{CClient, SignonState},
        cvar::convar::FCVAR_GAMEDLL,
    },
    prelude::*,
};
use shared::bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS};

use crate::{interfaces::ENGINE_INTERFACES, utils::iterate_c_array_sized};

use super::{choose_team, get_bot_name, spawn_fake_player};

static MAX_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
static MIN_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
static TARGET_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
static ENABLED_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();

#[derive(Debug, Clone)]
pub struct ManagerData {
    max: u32,
    target: u32,
    min: u32,
    pub enabled: bool,
    bots_to_spawn: u32,
    bots_to_remove: u32,
}

impl Default for ManagerData {
    fn default() -> Self {
        Self {
            max: 8,
            target: 4,
            min: 2,
            enabled: false,
            bots_to_spawn: 0,
            bots_to_remove: 0,
        }
    }
}

pub fn register_manager_sq_functions() {
    register_sq_functions(sq_bot_manager_max);
    register_sq_functions(sq_bot_manager_target);
    register_sq_functions(sq_bot_manager_min);
}

pub fn register_manager_vars(_: &EngineData, token: EngineToken) {
    _ = MAX_CONVAR.set(
        ConVarStruct::try_new(
            &ConVarRegister {
                callback: Some(bot_manager_max),
                ..ConVarRegister::mandatory(
                    "bot_manager_max",
                    "8",
                    FCVAR_GAMEDLL as i32,
                    "cut off limit when bots stop spawning",
                )
            },
            token,
        )
        .expect("couldn't register bot_manager_max"),
    );

    _ = TARGET_CONVAR.set(
        ConVarStruct::try_new(
            &ConVarRegister {
                callback: Some(bot_manager_target),
                ..ConVarRegister::mandatory(
                    "bot_manager_target",
                    "4",
                    FCVAR_GAMEDLL as i32,
                    "the minimum amount of total players which bots will try to fill",
                )
            },
            token,
        )
        .expect("couldn't register bot_manager_max"),
    );

    _ = TARGET_CONVAR.set(
        ConVarStruct::try_new(
            &ConVarRegister {
                callback: Some(bot_manager_min),
                ..ConVarRegister::mandatory(
                    "bot_manager_min",
                    "2",
                    FCVAR_GAMEDLL as i32,
                    "how many should be present after the target until max",
                )
            },
            token,
        )
        .expect("couldn't register bot_manager_max"),
    );

    _ = ENABLED_CONVAR.set(
        ConVarStruct::try_new(
            &ConVarRegister {
                callback: Some(bot_manager_enabled),
                ..ConVarRegister::mandatory(
                    "bot_manager_enabled",
                    "0",
                    FCVAR_GAMEDLL as i32,
                    "toggles the bot manager on and off",
                )
            },
            token,
        )
        .expect("couldn't register bot_manager_max"),
    );
}

pub fn check_player_amount(plugin: &super::Bots, token: EngineToken) -> Result<(), &'static str> {
    let engine_funcs = ENGINE_FUNCTIONS
        .get()
        .ok_or("failed to get engine functions")?;
    let server_funcs = SERVER_FUNCTIONS
        .get()
        .ok_or("failed to get server functions")?;
    let mut manager_data = plugin.manager_data.lock();

    let (real_players, fake_playes) = unsafe {
        iterate_c_array_sized::<_, 32>(engine_funcs.client_array.into())
            .filter(|client| client.signon.copy_inner() >= SignonState::CONNECTED)
            .fold((0u32, 0u32), |(real_players, fake_players), client| {
                (
                    real_players + client.fake_player.not() as u32,
                    fake_players + client.fake_player.copy_inner() as u32,
                )
            })
    };
    let total_players = real_players + fake_playes;

    manager_data.bots_to_spawn = if total_players < manager_data.target {
        manager_data.target - total_players
    } else if real_players >= manager_data.target
        && total_players < manager_data.max
        && total_players < manager_data.min + real_players
    {
        real_players + manager_data.min - total_players
    } else {
        0
    };
    manager_data.bots_to_remove = total_players.saturating_sub(manager_data.max);

    match (manager_data.bots_to_spawn, manager_data.bots_to_remove) {
        (1.., _) => {
            // add bots
            spawn_fake_player(
                get_bot_name(),
                choose_team(),
                None,
                server_funcs,
                engine_funcs,
                token,
            );
        }
        (0, r @ 1..) => {
            // remove extra bots
            let engine_server = ENGINE_INTERFACES.wait().engine_server;

            unsafe { engine_server.LockNetworkStringTables(true) };

            unsafe {
                iterate_c_array_sized::<_, 32>(engine_funcs.client_array.into())
                    .filter(|client| **client.signon == SignonState::FULL && **client.fake_player)
            }
            .take(r as usize)
            .for_each(|client| unsafe {
                (engine_funcs.cclient_disconnect)(
                    (client as *const CClient).cast_mut(),
                    1,
                    c"enough bots we have".as_ptr().cast(),
                )
            });

            unsafe { engine_server.LockNetworkStringTables(false) };
        }

        _ => {}
    }
    Ok(())
}

#[rrplug::convar]
fn bot_manager_enabled() -> Option<()> {
    crate::PLUGIN.wait().bots.manager_data.lock().enabled =
        ENABLED_CONVAR.get()?.get_value_i32().unsigned_abs() == 1;
    None
}

#[rrplug::convar]
fn bot_manager_max() -> Option<()> {
    crate::PLUGIN.wait().bots.manager_data.lock().max =
        MAX_CONVAR.get()?.get_value_i32().unsigned_abs();
    None
}

#[rrplug::convar]
fn bot_manager_target() -> Option<()> {
    crate::PLUGIN.wait().bots.manager_data.lock().target =
        TARGET_CONVAR.get()?.get_value_i32().unsigned_abs();
    None
}

#[rrplug::convar]
fn bot_manager_min() -> Option<()> {
    crate::PLUGIN.wait().bots.manager_data.lock().min =
        MIN_CONVAR.get()?.get_value_i32().unsigned_abs();
    None
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "SetBotManagerMax")]
fn sq_bot_manager_max(max: i32) {
    crate::PLUGIN.wait().bots.manager_data.lock().max = max.unsigned_abs();
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "SetBotManagerTarget")]
fn sq_bot_manager_target(target: i32) {
    crate::PLUGIN.wait().bots.manager_data.lock().target = target.unsigned_abs();
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "SetBotManagerMin")]
fn sq_bot_manager_min(min: i32) {
    crate::PLUGIN.wait().bots.manager_data.lock().min = min.unsigned_abs();
}
