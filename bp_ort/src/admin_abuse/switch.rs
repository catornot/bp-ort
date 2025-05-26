use rrplug::{
    bindings::{
        class_types::client::SignonState,
        cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
    },
    prelude::*,
};

use crate::{
    admin_abuse::{admin_check, execute_for_matches, forward_to_server},
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::{get_c_char_array, get_c_char_array_lossy, iterate_c_array_sized},
};

pub fn register_switch_command(engine_data: &EngineData, token: EngineToken) {
    _ = engine_data.register_concommand_with_completion(
        "switch",
        forward_to_server,
        "switch the team of the target",
        FCVAR_CLIENTDLL as i32,
        switch_completion,
        token,
    );

    _ = engine_data.register_concommand(
        "switch_server",
        switch_server_command,
        "",
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );

    _ = engine_data.register_concommand(
        "setteam",
        setteam,
        "",
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );
}

#[rrplug::concommand]
fn switch_server_command(command: CCommandResult) {
    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    let (check, admin) = admin_check(&command, engine, funcs);
    if !check {
        return;
    }

    let filter = command.get_arg(0).or_else(|| unsafe {
        admin.and_then(|admin| {
            get_c_char_array(
                &engine
                    .client_array
                    .add(admin.pl.index as usize - 1)
                    .as_ref()?
                    .m_szServerName,
            )
        })
    });

    execute_for_matches(
        filter,
        |player| player.m_iTeamNum = if player.m_iTeamNum == 2 { 3 } else { 2 },
        false,
        funcs,
        engine,
    );
}

#[rrplug::concommand]
fn setteam(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage:  {} < team >", command.get_command());
        return;
    }

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    let (is_admin, admin) = admin_check(&command, engine, funcs);
    if !is_admin {
        return;
    }

    if let (Some(player), Some(team)) = (admin, command.get_arg(0).unwrap_or_default().parse().ok())
    {
        player.m_iTeamNum = team;
    }
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
        .filter(|client| client.m_nSignonState == SignonState::FULL)
        .map(|client| get_c_char_array_lossy(&client.m_szServerName))
        .filter(|name| name.starts_with(current.partial))
        .for_each(|name| _ = suggestions.push(&format!("{} {}", current.cmd, name)));

    suggestions.commands_used()
}
