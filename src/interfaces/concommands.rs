use rrplug::bindings::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL};
use rrplug::prelude::*;

use crate::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::set_c_char_array,
};

pub fn register_concommands(engine: &EngineData) {
    engine
        .register_concommand(
            "disguise_name",
            disguise_name,
            "disguise_name <index> <name>",
            FCVAR_GAMEDLL as i32 | FCVAR_CLIENTDLL as i32,
        )
        .expect("couldn't register concommand bot_find");

    engine
        .register_concommand(
            "disguise_tag",
            disguise_tag,
            "disguise_name <index> <tag>",
            FCVAR_GAMEDLL as i32 | FCVAR_CLIENTDLL as i32,
        )
        .expect("couldn't register concommand bot_find");

    engine
        .register_concommand(
            "disguise_travesal",
            disguise_travesal,
            "disguise_travesal <index> <type>",
            FCVAR_GAMEDLL as i32,
        )
        .expect("couldn't register concommand bot_find");
}

#[rrplug::concommand]
pub fn disguise_name(command: CCommandResult) -> Option<()> {
    let index: usize = command.get_args().get(0)?.parse().ok()?;

    let name = command.get_args().get(1)?;

    let client = unsafe { ENGINE_FUNCTIONS.wait().client_array.add(index).as_mut()? };
    let player =
        unsafe { (SERVER_FUNCTIONS.wait().get_player_by_index)(index as i32 + 1).as_mut()? };

    unsafe {
        set_c_char_array(&mut client.name, name);
        set_c_char_array(&mut player.title, name);
        set_c_char_array(&mut player.community_name, name);
    }

    None
}

#[rrplug::concommand]
pub fn disguise_tag(command: CCommandResult) -> Option<()> {
    let index: usize = command.get_args().get(0)?.parse().ok()?;

    let tag = command.get_args().get(1)?;

    let client = unsafe { ENGINE_FUNCTIONS.wait().client_array.add(index).as_mut()? };
    let player =
        unsafe { (SERVER_FUNCTIONS.wait().get_player_by_index)(index as i32 + 1).as_mut()? };

    unsafe {
        set_c_char_array(&mut client.clan_tag, tag);
        set_c_char_array(&mut player.community_clan_tag, tag);
    }

    None
}

#[rrplug::concommand]
pub fn disguise_travesal(command: CCommandResult) -> Option<()> {
    unsafe {
        let index: i32 = command.get_args().get(0)?.parse().ok()?;

        let player = (SERVER_FUNCTIONS.wait().get_player_by_index)(index + 1).as_mut()?;

        log::info!("player.traversal_type {}", *player.traversal_type);

        let state: i32 = command.get_args().get(0)?.parse().ok()?;

        **player.traversal_type = state;
    }
    None
}
