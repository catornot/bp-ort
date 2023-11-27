use libc::c_char;
use rrplug::bindings::{
    class_types::{client::SignonState, cplayer::CPlayer},
    cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL, FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS},
};
use rrplug::prelude::*;

use crate::utils::{
    from_c_string, iterate_c_array_sized, register_concommand_with_completion, CommandCompletion,
    CurrentCommand,
};
use crate::{
    admin_abuse::get_admins,
    bindings::{ServerFunctions, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
};

pub fn register_slay_command(engine_data: &EngineData) {
    register_concommand_with_completion(
        engine_data,
        "slay",
        slay_command,
        "spawns a bot",
        FCVAR_CLIENTDLL as i32,
        slay_completion,
    );

    _ = engine_data.register_concommand(
        "slay_server",
        slay_server_command,
        "",
        // 0,
        FCVAR_GAMEDLL_FOR_REMOTE_CLIENTS as i32 | FCVAR_GAMEDLL as i32,
    );
}

#[rrplug::concommand]
pub fn slay_command(command: CCommandResult) {
    unsafe {
        let engine = ENGINE_FUNCTIONS.wait();
        let cmd = format!(
            "slay_server {}\0",
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
pub fn slay_server_command(command: CCommandResult) {
    if command.get_arg(0).is_none() {
        log::warn!("Usage:  {} < name >", command.get_command());
        return;
    }
    let engine = ENGINE_FUNCTIONS.wait();
    let funcs = SERVER_FUNCTIONS.wait();
    let is_dedicated = crate::PLUGIN.wait().is_dedicated_server();

    let has_admin = dbg!(unsafe { dbg!(*engine.cmd_source) != 1 || !is_dedicated })
        .then_some(unsafe {
            engine
                .host_client
                .as_ref()
                .map(|ptr| ptr.as_ref())
                .flatten()
                .map(|c| from_c_string::<String>(c.uid.as_ptr()))
        })
        .flatten()
        .map(|uid| get_admins().iter().any(|admin| admin.as_ref() == &uid))
        .unwrap_or(false);
    if !has_admin {
        log::warn!(
            "Client needs to have admin to run {}",
            command.get_command()
        );
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
        .for_each(|(player, _)| die_player(funcs, player));
}

fn filter_target(filter: Option<&str>, player: &CPlayer, name: &str) -> bool {
    match filter {
        Some("all") => true,
        Some("imc") => unsafe { *player.team.get_inner() == 2 },
        Some("militia") => unsafe { *player.team.get_inner() == 3 },
        Some(fname) => name.starts_with(fname),
        None => false,
    }
}

fn die_player(funcs: &ServerFunctions, player: &mut CPlayer) {
    unsafe { (funcs.set_health)(player, -1, 0, 0) }
}

pub extern "C" fn slay_completion(
    partial: *const c_char,
    commands: *mut [c_char;
        rrplug::bindings::cvar::convar::COMMAND_COMPLETION_ITEM_LENGTH as usize],
) -> i32 {
    let current = CurrentCommand::new(partial).unwrap();
    let mut suggestions = CommandCompletion::from(commands);

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
