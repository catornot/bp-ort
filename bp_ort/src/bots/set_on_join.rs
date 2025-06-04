use crate::{
    utils::{get_c_char_array_lossy, set_c_char_array},
    PLUGIN,
};
use rand::Rng;
use rrplug::{
    bindings::class_types::client::CClient,
    high::{engine::EngineToken, squirrel::call_sq_function},
    mid::squirrel::{SQFUNCTIONS, SQVM_SERVER},
};

const FUNNY_CLAN_TAGS: &[&str] = &[
    ">~<", "owo", "uwu", ":o", ":D", "ADV", "CLAN", "HI!", "PETAR", "<3", "BOB",
];

use super::UWUFY_CONVAR;

pub unsafe fn set_stuff_on_join(client: &mut CClient) {
    let name = get_c_char_array_lossy(&client.m_szServerName);
    let sqvm = SQVM_SERVER.get(EngineToken::new_unchecked()).borrow();
    let mut rng = rand::thread_rng();
    let plugin = PLUGIN.wait();

    if let Some((name, tag)) = plugin
        .bots
        .player_names
        .lock()
        .get(&client.m_UID)
        .filter(|_| !client.m_bFakePlayer) // do not allow fake players to use this sytem since they all have the the uid
        .filter(|(name, _)| name.is_ascii())
        .filter(|(_, tag)| tag.is_ascii())
    {
        log::info!(
            "found {name} and {tag} for {}",
            client
                .m_UID
                .as_slice()
                .iter()
                .filter_map(|i| char::from_u32(*i as u32))
                .filter(|c| *c != '\0')
                .collect::<String>()
        );
        set_c_char_array(&mut client.m_szServerName, name);
        set_c_char_array(&mut client.m_szClanTag, tag);
    }

    if client.m_bFakePlayer {
        set_c_char_array(
            &mut client.m_szClanTag,
            &crate::PLUGIN.wait().bots.clang_tag.lock(),
        );

        log::info!("set the clan tag for {name} bot");
    } else if name == "cat_or_not" {
        set_c_char_array(&mut client.m_szClanTag, "cat");
        set_c_char_array(&mut client.m_szServerName, "cat_or_nya");

        log::info!("set the clan tag for cat_or_not");
    } else if UWUFY_CONVAR.wait().get_value_bool() {
        log::info!("set the clan tag for {name}");

        let new_name = name.replace(['r', 'l'], "w").replace(['R', 'L'], "W");

        set_c_char_array(
            &mut client.m_szClanTag,
            FUNNY_CLAN_TAGS
                .get(rng.gen_range(0..FUNNY_CLAN_TAGS.len()))
                .copied()
                .unwrap_or_default(),
        );
        set_c_char_array(&mut client.m_szServerName, &new_name);
    } else if let Some(new_name) = sqvm.as_ref().and_then(|sqvm| {
        call_sq_function::<String, _>(
            *sqvm,
            SQFUNCTIONS.server.wait(),
            "CodeCallBack_CanChangeName",
            name,
        )
        .map_err(|err| err.log())
        .ok()
        .filter(|name| name.is_ascii())
    }) {
        set_c_char_array(&mut client.m_szServerName, new_name.as_str());
    }

    if let Some(new_tag) = sqvm.as_ref().and_then(|sqvm| {
        call_sq_function::<String, _>(
            *sqvm,
            SQFUNCTIONS.server.wait(),
            "CodeCallBack_CanChangeClangTag",
            (),
        )
        .map_err(|err| err.log())
        .ok()
        .and_then(|tag| if tag.len() < 12 { Some(tag) } else { None })
        .filter(|name| name.is_ascii())
    }) {
        set_c_char_array(&mut client.m_szClanTag, new_tag.as_str());
    }
}
