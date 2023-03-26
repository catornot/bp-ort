use rrplug::{bindings::convar::FCVAR_GAMEDLL, wrappers::engine::EngineData};

use crate::structs::cbaseplayer::CbasePlayer;

pub fn register_debug_concommands(engine: &EngineData) {
    engine
        .register_concommand(
            "peak_connected_clients",
            peak_connected_clients,
            "peaks the info about connected clients (dumps a lot of info)",
            FCVAR_GAMEDLL as i32,
        )
        .expect("couldn't register concommand peak_connected_clients");

    engine
        .register_concommand("bots_peak", bots_peak, "", FCVAR_GAMEDLL as i32)
        .expect("couldn't register concommand bots_peak");

    engine
        .register_concommand("bot_find", bot_find, "", FCVAR_GAMEDLL as i32)
        .expect("couldn't register concommand bot_find");

    engine
        .register_concommand(
            "bot_dump_players",
            bot_dump_players,
            "",
            FCVAR_GAMEDLL as i32,
        )
        .expect("couldn't register concommand bot_find");

    engine
        .register_concommand("set_clan_tag", set_clan_tag, "", FCVAR_GAMEDLL as i32)
        .expect("couldn't register concommand bot_find");
}

#[rrplug::concommand]
pub fn peak_connected_clients(command: CCommandResult) {
    crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .client_array
        .peak_array();
}

#[rrplug::concommand]
pub fn bot_find(command: CCommandResult) {
    let name = match command.args.get(0) {
        Some(n) => n,
        None => return,
    };

    let found_client = crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .client_array
        .find(|c| &c.get_name() == name);

    if let Some(c) = found_client {
        c.peak()
    }
}

#[rrplug::concommand]
pub fn bots_peak(command: CCommandResult) {
    for client in (&mut crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .client_array)
        .filter(|c| c.is_fake_player())
    {
        client.peak()
    }
}

#[rrplug::concommand]
pub fn bot_dump_players(command: CCommandResult) {
    let player_by_index = crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .player_by_index;
    for player in (0..32)
        .map(|i| unsafe { player_by_index(i + 1) })
        .filter_map(|ptr| CbasePlayer::try_from(ptr).ok())
    {
        log::info!(
            "player at index {} on team {}",
            player.get_index(),
            player.get_team()
        )
    }
}

#[rrplug::concommand]
pub fn set_clan_tag(command: CCommandResult) {
    let index = match command.args.get(0) {
        Some(index) => match index.parse::<i32>() {
            Ok(index) => index,
            _ => return,
        },
        None => return,
    };

    let tag = match command.args.get(0) {
        Some(tag) => tag.to_string(),
        None => return,
    };

    if let Ok(player) = CbasePlayer::try_from(unsafe {
        (crate::PLUGIN
            .wait()
            .source_engine_data
            .lock()
            .unwrap()
            .player_by_index)(index + 1)
    }) {
        player.set_clan_tag(tag)
    }

    

    // for client in (&mut crate::PLUGIN
    //     .wait()
    //     .source_engine_data
    //     .lock()
    //     .unwrap()
    //     .client_array)
    //     .filter(|c| c.get_signon() != SignonState::None)
    // {
    //     client.set_clan_tag(tag.clone())
    // }
}
