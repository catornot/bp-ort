use crate::{
    utils::{from_c_string, set_c_char_array},
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
    let name = from_c_string::<String>(&**client.name as *const i8);
    let sqvm = SQVM_SERVER.get(EngineToken::new_unchecked()).borrow();
    let mut rng = rand::thread_rng();
    let plugin = PLUGIN.wait();

    if let Some((name, tag)) = plugin
        .bots
        .player_names
        .lock()
        .get(&**client.uid)
        .filter(|_| !client.fake_player.copy_inner()) // do not allow fake players to use this sytem since they all have the the uid
        .filter(|(name, _)| name.is_ascii())
        .filter(|(_, tag)| tag.is_ascii())
    {
        log::info!(
            "found {name} and {tag} for {}",
            client
                .uid
                .as_slice()
                .iter()
                .filter_map(|i| char::from_u32(*i as u32))
                .filter(|c| *c != '\0')
                .collect::<String>()
        );
        set_c_char_array(&mut client.name, name);
        set_c_char_array(&mut client.clan_tag, tag);
    }

    if *client.fake_player.get_inner() {
        set_c_char_array(
            &mut client.clan_tag,
            &crate::PLUGIN.wait().bots.clang_tag.lock(),
        );

        log::info!("set the clan tag for {} bot", name);
    } else if name == "cat_or_not" {
        set_c_char_array(&mut client.clan_tag, "cat");
        set_c_char_array(&mut client.name, "cat_or_nya");

        log::info!("set the clan tag for cat_or_not");
    } else if UWUFY_CONVAR.wait().get_value_bool() {
        log::info!("set the clan tag for {name}");

        let new_name = name.replace(['r', 'l'], "w").replace(['R', 'L'], "W");

        set_c_char_array(
            &mut client.clan_tag,
            FUNNY_CLAN_TAGS
                .get(rng.gen_range(0..FUNNY_CLAN_TAGS.len()))
                .copied()
                .unwrap_or_default(),
        );
        set_c_char_array(&mut client.name, &new_name);
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
        set_c_char_array(&mut client.name, new_name.as_str());
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
        set_c_char_array(&mut client.clan_tag, new_tag.as_str());
    }
}
