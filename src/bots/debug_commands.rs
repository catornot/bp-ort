use rrplug::bindings::cvar::convar::FCVAR_GAMEDLL;
use rrplug::prelude::*;
use std::ffi::CStr;

use crate::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    // bots::navmesh::get_path,
    utils::iterate_c_array_sized,
};

pub fn register_debug_concommands(engine: &EngineData, token: EngineToken) {
    engine
        .register_concommand("bot_find", bot_find, "", FCVAR_GAMEDLL as i32, token)
        .expect("couldn't register concommand bot_find");

    engine
        .register_concommand(
            "bot_dump_players",
            bot_dump_players,
            "",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand bot_dump_players");

    engine
        .register_concommand(
            "set_clan_tag",
            set_clan_tag,
            "",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand set_clan_tag");

    engine
        .register_concommand("test_nav", test_nav, "", FCVAR_GAMEDLL as i32, token)
        .expect("couldn't register concommand test_nav");
}

#[rrplug::concommand]
pub fn bot_find(command: CCommandResult) {
    let name = match command.get_arg(0) {
        Some(n) => n,
        None => return,
    };

    let found_client = unsafe {
        iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into())
            .map(|c| {
                CStr::from_ptr(c.name.as_ref() as *const [i8] as *const i8)
                    .to_string_lossy()
                    .to_string()
            })
            .find(|n| n == name)
    };

    if let Some(c) = found_client {
        log::info!("found bot {c}");
    }
}

#[rrplug::concommand]
pub fn bot_dump_players() {
    for player in (0..32)
        .map(|i| unsafe { (SERVER_FUNCTIONS.wait().get_player_by_index)(i + 1) })
        .filter_map(|ptr| unsafe { ptr.as_ref() })
    {
        unsafe {
            log::info!(
                "player at index {:?} on team {:?}",
                player.player_index,
                player.team,
            )
        }
    }
}

#[rrplug::concommand]
pub fn set_clan_tag(command: CCommandResult) {
    let index = match command.get_arg(0) {
        Some(index) => match index.parse::<i32>() {
            Ok(index) => index,
            _ => return,
        },
        None => return,
    };

    let tag = match command.get_arg(0) {
        Some(tag) => tag.bytes(),
        None => return,
    };

    log::info!("setting clan tag");

    match unsafe { (SERVER_FUNCTIONS.wait().get_player_by_index)(index + 1).as_mut() } {
        Some(player) => unsafe {
            player
                .community_clan_tag
                .iter_mut()
                .zip(tag)
                .for_each(|(c, tag_c)| *c = tag_c as i8)
        },
        None => log::info!("failed to find the player"),
    }
}

#[rrplug::concommand]
pub fn test_nav() {
    // unsafe {
    //     get_path(Vector3::new(0., 10., 0.), Vector3::new(100., 10., 0.));
    // }
}
