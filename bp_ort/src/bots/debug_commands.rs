use rrplug::{
    bindings::cvar::convar::FCVAR_GAMEDLL,
    mid::utils::{from_char_ptr, to_cstring, try_cstring},
    prelude::*,
};
use shared::utils::get_player_index;

use crate::{
    admin_abuse::execute_for_matches,
    utils::{get_c_char_array_lossy, get_ents_by_class_name, get_weaponx_name, lookup_ent},
};
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

    engine
        .register_concommand(
            "bot_run_cmd",
            bot_run_cmd,
            "runs a client command on the bot",
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
            .map(|c| get_c_char_array_lossy(&c.m_szServerName))
            .find(|n| n == name)
    };

    if let Some(c) = found_client {
        log::info!("found bot {c}");
    }
}

#[rrplug::concommand]
pub fn bot_dump_players() {
    let server_funcs = SERVER_FUNCTIONS.wait();
    let engine_funcs = ENGINE_FUNCTIONS.wait();
    for (player, client) in (0..32)
        .map(|i| unsafe {
            (
                (server_funcs.get_player_by_index)(i + 1),
                engine_funcs.client_array.add(i as usize),
            )
        })
        .filter_map(|(player, client)| unsafe { Some((player.as_ref()?, client.as_ref()?)) })
    {
        if client.m_bFakePlayer {
            let data = super::BOT_DATA_MAP.get(engine_token).try_borrow().ok();
            log::info!(
                "{}: {} on team {} with sim_type {} with titan {:?}",
                unsafe { from_char_ptr((SERVER_FUNCTIONS.wait().get_entity_name)(player)) },
                get_player_index(player),
                player.m_iTeamNum,
                data.as_ref()
                    .and_then(|data| data.get(client.m_nHandle as usize)?.sim_type)
                    .unwrap_or(-1),
                data.and_then(|data| Some(data.get(client.m_nHandle as usize)?.titan))
                    .unwrap_or_default(),
            );
        } else {
            log::info!(
                "{}: {} on team {}",
                unsafe { from_char_ptr((SERVER_FUNCTIONS.wait().get_entity_name)(player)) },
                get_player_index(player),
                player.m_iTeamNum,
            );
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
    for (index, player, signon) in (0..32).filter_map(|index| unsafe {
        Some((
            index,
            get_c_char_array_lossy(
                &(engine_funcs.client_array.add(index))
                    .as_ref()?
                    .m_szServerName,
            ),
            (engine_funcs.client_array.add(index))
                .as_ref()?
                .m_nSignonState,
        ))
    }) {
        log::info!("{index}: {player}, {signon:?}");
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

    let weapon_id = match command
        .get_arg(0)
        .and_then(|index| index.parse::<usize>().ok())
        .unwrap_or(usize::MAX)
    {
        index if matches!(index, 0..4) => player.m_inventory.weapons[index],
        index if matches!(index, 4..6) => player.m_inventory.offhandWeapons[index - 4],
        _ => player.m_inventory.activeWeapon,
    };

    let ent = lookup_ent(weapon_id, server)?;
    let name = ent.m_iClassname as usize as *const i8;
    if !name.is_null() && ent.m_iClassname != 0xffff && ent.m_iClassname != 0xff {
        log::info!("weapon name {}", unsafe {
            rrplug::mid::utils::from_char_ptr(name)
        });
        log::info!("weapon name {}", get_weaponx_name(ent, server)?);
    }

    None
}

#[rrplug::concommand]
pub fn bot_run_cmd(command: CCommandResult) -> Option<()> {
    let server = SERVER_FUNCTIONS.wait();
    let engine = ENGINE_FUNCTIONS.wait();

    let sqvm = mid::squirrel::SQVM_SERVER
        .get(unsafe { EngineToken::new_unchecked() })
        .borrow();
    let sqvm = sqvm.as_ref()?;

    let commmand_arg = command.get_arg(1)?;
    let mut args = vec![commmand_arg.to_owned()];
    args.extend_from_slice(command.get_args().get(2..).unwrap_or_default());

    let exec = |player: &mut rrplug::bindings::class_types::cplayer::CPlayer| {
        log::info!(
            "running {commmand_arg} for {} with {:?}",
            unsafe { from_char_ptr((server.get_entity_name)(player)) },
            command.get_args().get(2..).unwrap_or_default()
        );

        high::squirrel::call_sq_function::<(), _>(
            *sqvm,
            SQFUNCTIONS.server.wait(),
            "CodeCallback_ClientCommand",
            (unsafe { high::UnsafeHandle::new(&*player) }, args.clone()),
        )
        .unwrap_or_default()
    };

    match command
        .get_arg(0)?
        .parse::<usize>()
        .ok()
        .and_then(|i| unsafe { (server.get_player_by_index)(i as i32).as_mut() })
    {
        None => execute_for_matches(Some(command.get_arg(0)?), exec, false, server, engine),
        Some(player) => exec(player),
    }

    None
}
