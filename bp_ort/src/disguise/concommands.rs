use rrplug::bindings::cvar::convar::{FCVAR_CLIENTDLL, FCVAR_GAMEDLL};
use rrplug::prelude::*;
use shared::utils::get_c_char_array;

use crate::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::set_c_char_array,
};

pub fn register_concommands(engine: &EngineData, token: EngineToken) {
    engine
        .register_concommand(
            "disguise_name",
            disguise_name,
            "disguise_name <index> <name>",
            FCVAR_GAMEDLL as i32 | FCVAR_CLIENTDLL as i32,
            token,
        )
        .expect("couldn't register concommand disguise_name");

    engine
        .register_concommand(
            "disguise_tag",
            disguise_tag,
            "disguise_name <index> <tag>",
            FCVAR_GAMEDLL as i32 | FCVAR_CLIENTDLL as i32,
            token,
        )
        .expect("couldn't register concommand disguise_tag");

    engine
        .register_concommand(
            "disguise_travesal",
            disguise_travesal,
            "disguise_travesal <index> <type>",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand disguise_travesal");

    engine
        .register_concommand(
            "disguise_edict",
            disguise_edict,
            "disguise_edict <index> <edict>",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand disguise_edict");

    engine
        .register_concommand(
            "disguise_generation",
            disguise_generation,
            "disguise_edict <index> <generation>",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand disguise_generation");
}

#[rrplug::concommand]
pub fn disguise_name(command: CCommandResult) -> Option<()> {
    let index: usize = command.get_arg(0)?.parse().ok()?;

    let name = command.get_args().get(1)?;

    let client = unsafe { ENGINE_FUNCTIONS.wait().client_array.add(index).as_mut()? };

    unsafe {
        set_c_char_array(&mut client.m_szServerName, "");
        (ENGINE_FUNCTIONS.wait().cclient_setname)(
            client,
            (name.to_string() + "\0").as_ptr().cast(),
        );
    }

    None
}

#[rrplug::concommand]
pub fn disguise_tag(command: CCommandResult) -> Option<()> {
    let index: usize = command.get_arg(0)?.parse().ok()?;

    let tag = command.get_args().get(1)?;

    let client = unsafe { ENGINE_FUNCTIONS.wait().client_array.add(index).as_mut()? };
    let player =
        unsafe { (SERVER_FUNCTIONS.wait().get_player_by_index)(index as i32 + 1).as_mut()? };

    let name = get_c_char_array(&client.m_szServerName)
        .unwrap_or("null")
        .to_string();
    unsafe {
        // HACK: setting the player name also updates the clan tag
        set_c_char_array(&mut client.m_szServerName, "");
        set_c_char_array(&mut client.m_szClanTag, tag);
        set_c_char_array(&mut player.m_communityClanTag, tag);
        (ENGINE_FUNCTIONS.wait().cclient_setname)(client, (name + "\0").as_ptr().cast());
    }

    None
}

#[rrplug::concommand]
pub fn disguise_travesal(command: CCommandResult) -> Option<()> {
    unsafe {
        let index: i32 = command.get_arg(0)?.parse().ok()?;

        let player = (SERVER_FUNCTIONS.wait().get_player_by_index)(index + 1).as_mut()?;

        log::info!("player.traversal_type {}", player.m_traversalType);

        let state: i32 = command.get_arg(0)?.parse().ok()?;

        player.m_traversalType = state;
    }
    None
}

#[rrplug::concommand]
pub fn disguise_edict(command: CCommandResult) -> Option<()> {
    unsafe {
        let index: usize = command.get_arg(0)?.parse().ok()?;

        let client = ENGINE_FUNCTIONS.wait().client_array.add(index).as_mut()?;

        log::info!("client.handle {}", client.m_nHandle);

        let edict: u16 = command.get_arg(0)?.parse().ok()?;

        client.m_nHandle = edict;
    }
    None
}

#[rrplug::concommand]
pub fn disguise_generation(command: CCommandResult) -> Option<()> {
    unsafe {
        let index: usize = command.get_arg(0)?.parse().ok()?;

        let player = (SERVER_FUNCTIONS.wait().get_player_by_index)(index as i32 + 1).as_mut()?;

        log::info!("player.generation {}", player.m_generation);

        let generation: i32 = command.get_arg(0)?.parse().ok()?;

        player.m_generation = generation;
    }
    None
}
