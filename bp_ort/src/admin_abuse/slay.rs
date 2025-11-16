use rrplug::bindings::{
    class_types::cplayer::CPlayer,
    cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
};
use rrplug::prelude::*;

use crate::{
    admin_abuse::{
        admin_check, completion_append_player_names, execute_for_matches, forward_to_server,
    },
    bindings::{ServerFunctions, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
};

pub fn register_slay_command(engine_data: &EngineData, token: EngineToken) {
    _ = engine_data.register_concommand_with_completion(
        "slay",
        forward_to_server,
        "kills a desired target",
        FCVAR_CLIENTDLL as i32,
        slay_completion,
        token,
    );

    _ = engine_data.register_concommand(
        "slay_server",
        slay_server_command,
        "",
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );
}

#[rrplug::concommand]
fn slay_server_command(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage:  {} < name >", command.get_command());
        return;
    }

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    if !admin_check(&command, engine, funcs).0 {
        return;
    }

    execute_for_matches(
        command.get_arg(0),
        |player| die_player(funcs, player),
        true,
        funcs,
        engine,
    );
}

fn die_player(funcs: &ServerFunctions, player: &mut CPlayer) {
    unsafe { (funcs.set_health)(player, -1, 0, 0) }
}

#[rrplug::completion]
fn slay_completion(current: CurrentCommand, suggestions: CommandCompletion) -> i32 {
    if "all".starts_with(current.partial) {
        _ = suggestions.push(&format!("{} all", current.cmd))
    }

    if "imc".starts_with(current.partial) {
        _ = suggestions.push(&format!("{} imc", current.cmd))
    }

    if "militia".starts_with(current.partial) {
        _ = suggestions.push(&format!("{} militia", current.cmd))
    }

    completion_append_player_names(current.partial, |name| {
        _ = suggestions.push(&format!("{} {}", current.cmd, name))
    });

    suggestions.commands_used()
}
