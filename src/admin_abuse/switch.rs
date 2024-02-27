use rrplug::{
    bindings::{
        class_types::client::SignonState,
        cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
    },
    prelude::*,
};
use std::ffi::c_char;

use crate::{
    admin_abuse::{admin_check, filter_target},
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::{from_c_string, iterate_c_array_sized},
};

pub fn register_switch_command(engine_data: &EngineData, token: EngineToken) {
    _ = engine_data.register_concommand_with_completion(
        "switch",
        switch_command,
        "switch the team of the target",
        FCVAR_CLIENTDLL as i32,
        switch_completion,
        token,
    );

    _ = engine_data.register_concommand(
        "switch_server",
        switch_server_command,
        "",
        // 0,
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );

    _ = engine_data.register_concommand(
        "setteam",
        setteam,
        "",
        // 0,
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );
}

#[rrplug::concommand]
pub fn switch_command(command: CCommandResult) {
    unsafe {
        let engine = ENGINE_FUNCTIONS.wait();
        let cmd = format!(
            "switch_server {}\0",
            command
                .get_args()
                .iter()
                .cloned()
                .map(|s| s + " ")
                .collect::<String>()
        );
        let cmd_ptr = cmd.as_ptr() as *const c_char;

        (engine.cengine_client_server_cmd)(std::ptr::null_mut(), cmd_ptr, true);
    }
}

#[rrplug::concommand]
pub fn switch_server_command(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage:  {} < name >", command.get_command());
        return;
    }
    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    if !admin_check(&command, engine, funcs).0 {
        return;
    }

    unsafe { iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()) }
        .enumerate()
        .filter(|(_, client)| unsafe { *client.signon.get_inner() } == SignonState::FULL)
        .filter_map(|(e, client)| unsafe {
            Some((
                (funcs.get_player_by_index)(e as i32 + 1).as_mut()?,
                from_c_string::<String>(client.name.get_inner().as_ptr()),
            ))
        })
        .filter(|(player, _)| unsafe { (funcs.is_alive)(*player) != 0 })
        .filter(|(player, name)| filter_target(command.get_arg(0), player, name))
        .for_each(|(player, _)| unsafe {
            *player.team.get_inner_mut() = if player.team.copy_inner() == 2 { 3 } else { 2 }
        });
}

#[rrplug::concommand]
pub fn setteam(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage:  {} < team >", command.get_command());
        return;
    }

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    let is_admin = admin_check(&command, engine, funcs);
    if !is_admin.0 {
        return;
    }

    _ = is_admin.1.map(|player| {
        unsafe { *player.team.get_inner_mut() = command.get_arg(0)?.parse().ok()? }
        Some(())
    });
}

#[rrplug::completion]
fn switch_completion(current: CurrentCommand, suggestions: CommandCompletion) -> i32 {
    // let get_player_by_index = CLIENT_FUNCTIONS.wait().get_c_player_by_index;

    if "all".starts_with(current.partial) {
        _ = suggestions.push(&format!("{} all", current.cmd))
    }

    if "imc".starts_with(current.partial) {
        _ = suggestions.push(&format!("{} imc", current.cmd))
    }

    if "militia".starts_with(current.partial) {
        _ = suggestions.push(&format!("{} militia", current.cmd))
    }

    unsafe { iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()) }
        .filter(|client| unsafe { *client.signon.get_inner() } == SignonState::FULL)
        .map(|client| unsafe { from_c_string::<String>(client.name.get_inner().as_ptr()) })
        .filter(|name| name.starts_with(current.partial))
        .for_each(|name| _ = suggestions.push(&format!("{} {}", current.cmd, name)));

    suggestions.commands_used()
}
