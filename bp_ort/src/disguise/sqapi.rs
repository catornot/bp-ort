use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};
use shared::utils::{get_c_char_array, get_player_index};

use crate::{bindings::ENGINE_FUNCTIONS, utils::set_c_char_array};

pub fn disguise_sqapi() {
    register_sq_functions(disguise_name);
    register_sq_functions(disguise_tag);
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

    if name.len() >= client.m_szServerName.len()
        || name.is_char_boundary(client.m_szServerName.len() - 1)
    {
        Err("too long")?;
    }

    unsafe {
        // HACK: setting player name to nothing tricks the game into running setname
        set_c_char_array(&mut player.m_szNetname, &name);
        set_c_char_array(&mut client.m_szServerName, "");
        (ENGINE_FUNCTIONS.wait().cclient_setname)(
            client,
            (name.to_string() + "\0").as_ptr().cast(),
        );
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

    if tag.len() >= client.m_szServerName.len()
        || tag.is_char_boundary(client.m_szServerName.len() - 1)
    {
        Err("too long")?;
    }

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
