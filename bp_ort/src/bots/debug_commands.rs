use rrplug::{
    bindings::cvar::convar::FCVAR_GAMEDLL,
    mid::utils::{from_char_ptr, to_cstring, try_cstring},
    prelude::*,
};
use std::ffi::CStr;

use crate::utils::{get_ents_by_class_name, get_weaponx_name, lookup_ent};
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
            "sets clan tag: set_clang_tag <player:0 to 32>",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand set_clan_tag");

    engine
        .register_concommand(
            "test_net_int",
            test_net_int,
            "test_net_int",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand test_net_int");

    engine
        .register_concommand(
            "bot_list_player_indicies",
            bot_list_player_indicies,
            "lists all the players and their index",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand bot_list_player_indicies");

    engine
        .register_concommand(
            "bot_find_ents_by_class",
            bot_find_ents_by_class,
            "",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand bot_find_ents_by_class");

    engine
        .register_concommand(
            "bot_get_weapon_class",
            bot_get_weapon_class,
            "",
            FCVAR_GAMEDLL as i32,
            token,
        )
        .expect("couldn't register concommand bot_get_weapon_class");
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
        log::info!(
            "player at index {} on team {} {}",
            player.pl.index,
            player.m_iTeamNum,
            player.m_boostMeter,
        );
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

    let tag = match command.get_arg(1) {
        Some(tag) => tag.bytes(),
        None => return,
    };

    log::info!("setting clan tag");

    match unsafe { (SERVER_FUNCTIONS.wait().get_player_by_index)(index + 1).as_mut() } {
        Some(player) => player
            .m_communityClanTag
            .iter_mut()
            .zip(tag)
            .for_each(|(c, tag_c)| *c = tag_c as i8),
        None => log::info!("failed to find the player"),
    }
}

#[rrplug::concommand]
pub fn test_net_int(command: CCommandResult) -> Option<()> {
    let index = command.get_arg(0)?.parse::<i32>().ok()?;

    let net_var = command.get_arg(1)?;

    let server_funcs = SERVER_FUNCTIONS.wait();

    log::info!("{net_var}: {}", unsafe {
        (server_funcs.get_player_by_index)(index + 1)
            .as_mut()
            .and_then(|player| {
                log::info!(
                    "pet_titan: {:?}",
                    // str_from_char_ptr((server_funcs.get_entity_name)(lookup_ent(
                    //     player.pet_titan.copy_inner(),
                    //     server_funcs
                    // )?
                    //     as *const _
                    //     as *const CPlayer))
                    dbg!(lookup_ent(player.m_petTitan, server_funcs,)? as *const _ as usize)
                        == (server_funcs.get_pet_titan)(player) as usize
                );
                Some((server_funcs.get_player_net_int)(
                    player,
                    try_cstring(net_var).ok()?.as_ptr(),
                ))
            })
    }?);

    None
}

#[rrplug::concommand]
pub fn bot_list_player_indicies() {
    let engine_funcs = ENGINE_FUNCTIONS.wait();
    for (index, player) in (0..32).filter_map(|index| {
        Some((index, unsafe {
            from_char_ptr(
                (engine_funcs.client_array.add(index))
                    .as_ref()?
                    .name
                    .as_ptr(),
            )
        }))
    }) {
        log::info!("{index}: {player}");
    }
}

#[rrplug::concommand]
pub fn bot_find_ents_by_class(command: CCommandResult) -> Option<()> {
    let mut v = Vector3::ZERO;

    get_ents_by_class_name(
        to_cstring(command.get_arg(0)?).as_c_str(),
        SERVER_FUNCTIONS.wait(),
    )
    .map(|ent| unsafe {
        *ent.cast::<rrplug::bindings::class_types::cplayer::CPlayer>()
            .as_ref()
            .unwrap_unchecked()
            .get_origin(&mut v)
    })
    .for_each(|pos| log::info!("found ent at {pos:?}"));

    None
}

#[rrplug::concommand]
pub fn bot_get_weapon_class(command: CCommandResult) -> Option<()> {
    let server = SERVER_FUNCTIONS.wait();
    let engine = ENGINE_FUNCTIONS.wait();
    let player = crate::admin_abuse::admin_check(&command, engine, server).1?;
    let ent = lookup_ent(player.m_inventory.activeWeapon, server)?;
    let name = ent.m_iClassname as usize as *const i8;
    if !name.is_null() && ent.m_iClassname != 0xffff && ent.m_iClassname != 0xff {
        log::info!("weapon name {}", unsafe {
            rrplug::mid::utils::from_char_ptr(name)
        });
        log::info!("weapon name {}", get_weaponx_name(ent, server, engine)?);
    }

    None
}
