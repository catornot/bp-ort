use rrplug::bindings::{
    class_types::{client::SignonState, cplayer::CPlayer},
    cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
};
use rrplug::prelude::*;

use crate::{
    admin_abuse::{admin_check, execute_for_matches, forward_to_server},
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::{from_c_string, iterate_c_array_sized, send_client_print},
};

pub fn register_health_command(engine_data: &EngineData, token: EngineToken) {
    _ = engine_data.register_concommand_with_completion(
        "health",
        forward_to_server,
        "kills a desired target",
        FCVAR_CLIENTDLL as i32,
        health_completion,
        token,
    );

    _ = engine_data.register_concommand(
        "health_server",
        health_server_command,
        "",
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );
}

#[rrplug::concommand]
fn health_server_command(command: CCommandResult) {
    if command.get_arg(1).is_none() {
        log::warn!("Usage:  {} < name > < health >", command.get_command());
        return;
    }

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    let (is_admin, maybe_admin) = admin_check(&command, engine, funcs);
    if !is_admin {
        return;
    }

    let Some(health) = command
        .get_arg(1)
        .and_then(|health| health.parse::<i32>().ok())
    else {
        if let Some(admin) = maybe_admin {
            send_client_print(admin, "health: input health amount");
        }

        return;
    };

    execute_for_matches(
        command.get_arg(0),
        |player| set_health(player, health),
        true,
        funcs,
        engine,
    );
}

fn set_health(player: &mut CPlayer, mut health: i32) {
    if health >= 524286 {
        health = 524286;
    }

    player.m_iMaxHealth = health;
    player.m_iHealth = health;
}

#[rrplug::completion]
fn health_completion(current: CurrentCommand, suggestions: CommandCompletion) {
    let Some((prev, _)) = current.partial.split_once(' ') else {
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
        return;
    };

    // common health amount
    [12000, 100, 1, 10, 524286]
        .into_iter()
        .for_each(|health| _ = suggestions.push(&format!("{} {} {}", current.cmd, prev, health)))
}
