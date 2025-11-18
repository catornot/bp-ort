use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};
use shared::utils::{get_c_char_array, get_player_index};

use crate::{bindings::ENGINE_FUNCTIONS, utils::set_c_char_array};

pub fn digsuise_sqapi() {
    register_sq_functions(disguise_name);
    register_sq_functions(disguise_tag);
    register_sq_functions(disguise_edict);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "DisguiseName")]
pub fn disguise_name(player: Option<&mut CPlayer>, name: String) -> Result<(), String> {
    let Some(player) = player else {
        return Err("I am sorry but this is not a player")?; // real
    };

    let client = unsafe {
        ENGINE_FUNCTIONS
            .wait()
            .client_array
            .add(get_player_index(player))
            .as_mut()
            .ok_or("cannot find the corresponding client for the player")?
    };

    unsafe {
        set_c_char_array(&mut client.m_szServerName, "");
        (ENGINE_FUNCTIONS.wait().cclient_setname)(client, (name + "\0").as_ptr().cast());
    }

    Ok(())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "DisguiseTag")]
pub fn disguise_tag(player: Option<&mut CPlayer>, tag: String) -> Result<(), String> {
    let Some(player) = player else {
        return Err("I am sorry but this is not a player")?; // real
    };

    let client = unsafe {
        ENGINE_FUNCTIONS
            .wait()
            .client_array
            .add(get_player_index(player))
            .as_mut()
            .ok_or("cannot find the corresponding client for the player")?
    };

    let name = get_c_char_array(&client.m_szServerName)
        .unwrap_or("null")
        .to_string();
    unsafe {
        // HACK: setting the player name also updates the clan tag
        set_c_char_array(&mut client.m_szServerName, "");
        set_c_char_array(&mut client.m_szClanTag, &tag);
        set_c_char_array(&mut player.m_communityClanTag, &tag);
        (ENGINE_FUNCTIONS.wait().cclient_setname)(client, (name + "\0").as_ptr().cast());
    }

    Ok(())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "DisguiseEdict")]
pub fn disguise_edict(player: Option<&mut CPlayer>, edict: i32) -> Result<(), String> {
    let Some(player) = player else {
        return Err("I am sorry but this is not a player")?; // real
    };

    let client = unsafe {
        ENGINE_FUNCTIONS
            .wait()
            .client_array
            .add(get_player_index(player))
            .as_mut()
            .ok_or("cannot find the corresponding client for the player")?
    };

    client.m_nHandle = edict
        .try_into()
        .ok()
        .ok_or("it's an u16 not whatever this is")?;

    Ok(())
}
