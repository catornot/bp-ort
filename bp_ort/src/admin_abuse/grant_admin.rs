use rrplug::prelude::*;
use rrplug::{
    bindings::{
        class_types::{client::SignonState, cplayer::CPlayer},
        cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
    },
    mid::utils::str_from_char_ptr,
};

use crate::admin_abuse::parse_admins;
use crate::{
    admin_abuse::{admin_check, execute_for_matches, forward_to_server},
    bindings::{EngineFunctions, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::{from_c_string, iterate_c_array_sized},
};

pub fn register_grant_admin_command(engine_data: &EngineData, token: EngineToken) {
    _ = engine_data.register_concommand_with_completion(
        "grant__admin",
        forward_to_server,
        "gives admin to a target",
        FCVAR_CLIENTDLL as i32,
        grant_admin_completion,
        token,
    );

    _ = engine_data.register_concommand(
        "grant__admin_server",
        grant_admin_server_command,
        "",
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
        token,
    );
}

#[rrplug::concommand]
fn grant_admin_server_command(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage: {} < name >", command.get_command());
        return;
    }

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    if !admin_check(&command, engine, funcs).0 {
        return;
    }

    // should only have one match
    execute_for_matches(
        command.get_arg(0),
        |player| _ = add_admin(player, engine, engine_token),
        false,
        funcs,
        engine,
    );

    _ = ConVarStruct::find_convar_by_name("grant_admin", engine_token).map(parse_admins)
}

fn add_admin(player: &CPlayer, engine_funcs: &EngineFunctions, token: EngineToken) -> Option<()> {
    let uid = unsafe {
        let client = engine_funcs
            .client_array
            .add(player.pl.index as usize - 1)
            .as_ref()?;
        let uid = client.uid.as_ptr();
        str_from_char_ptr(uid)?
    };

    let convar = ConVarStruct::find_convar_by_name("grant_admin", token).ok()?;

    convar.set_value_string(format!("{},{}", convar.get_value_string(), uid), token);

    Some(())
}

#[rrplug::completion]
fn grant_admin_completion(current: CurrentCommand, suggestions: CommandCompletion) -> i32 {
    unsafe { iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()) }
        .filter(|client| unsafe { *client.signon.get_inner() } == SignonState::FULL)
        .map(|client| unsafe { from_c_string::<String>(client.name.get_inner().as_ptr()) })
        .filter(|name| name.starts_with(current.partial))
        .for_each(|name| _ = suggestions.push(&format!("{} {}", current.cmd, name)));

    suggestions.commands_used()
}
