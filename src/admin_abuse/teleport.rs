#![allow(unused)]

use rrplug::prelude::*;
use rrplug::{
    bindings::{
        class_types::client::SignonState,
        cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
    },
    mid::utils::from_char_ptr,
};

use crate::bindings::CLIENT_FUNCTIONS;
use crate::{
    admin_abuse::{admin_check, execute_for_matches, forward_to_server},
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::{from_c_string, iterate_c_array_sized},
};

pub fn register_teleport_command(engine_data: &EngineData, token: EngineToken) {
    // _ = engine_data.register_concommand_with_completion(
    //     "tp",
    //     forward_to_server,
    //     "kills a desired target",
    //     FCVAR_CLIENTDLL as i32,
    //     teleport_completion,
    //     token,
    // );

    // _ = engine_data.register_concommand(
    //     "tp_server",
    //     teleport_server_command,
    //     "",
    //     FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
    //     token,
    // );

    _ = engine_data.register_concommand(
        "pos",
        print_player_position,
        "",
        FCVAR_CLIENTDLL as i32,
        token,
    );
}

#[rrplug::concommand]
fn print_player_position() -> Option<()> {
    log::info!("pos: {:?}", unsafe {
        *(CLIENT_FUNCTIONS.wait().get_local_c_player)()
            .as_ref()?
            .get_origin()
    });

    None
}

#[rrplug::concommand]
fn teleport_server_command(command: CCommandResult) -> Option<()> {
    if command.get_arg(1).is_none() {
        log::warn!("Usage:  {} < name > < name >", command.get_command());
        return None;
    }

    let name_target = command.get_arg(1)?;

    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();

    if !admin_check(&command, engine, funcs).0 {
        return None;
    }

    let mut v = Vector3::ZERO;
    let tp_location = unsafe { iterate_c_array_sized::<_, 32>(engine.client_array.into()) }
        .enumerate()
        .filter(|(_, client)| unsafe { *client.signon.get_inner() } == SignonState::FULL)
        .filter_map(|(e, client)| unsafe {
            Some((
                (funcs.get_player_by_index)(e as i32 + 1).as_mut()?,
                from_char_ptr(client.name.get_inner().as_ptr()),
            ))
        })
        .find_map(|(player, name)| {
            name_target
                .starts_with(name.as_str())
                .then_some(unsafe { *player.get_origin(&mut v) })
        })?;

    // teleporting this way doesn't always work :\

    execute_for_matches(
        command.get_arg(0),
        |player| unsafe { *player.local_origin.get_inner_mut() = tp_location },
        false,
        funcs,
        engine,
    );

    Some(())
}

#[rrplug::completion]
fn teleport_completion(current: CurrentCommand, suggestions: CommandCompletion) {
    let Some((prev, next)) = current.partial.split_once(' ') else {
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

    unsafe { iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()) }
        .filter(|client| unsafe { *client.signon.get_inner() } == SignonState::FULL)
        .map(|client| unsafe { from_c_string::<String>(client.name.get_inner().as_ptr()) })
        .filter(|name| name.starts_with(next))
        .for_each(|name| _ = suggestions.push(&format!("{} {} {}", current.cmd, prev, name)));
}
