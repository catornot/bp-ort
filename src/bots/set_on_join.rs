use crate::utils::{from_c_string, set_c_char_array};
use rrplug::bindings::entity::CBaseClient;

pub unsafe fn set_stuff_on_join(client: &mut CBaseClient) {
    let name = from_c_string::<String>(&**client.name as *const i8);

    if *client.fake_player.get_inner() {
        set_c_char_array(
            &mut client.clan_tag,
            &crate::PLUGIN.wait().bots.clang_tag.lock().expect("how"),
        );

        log::info!("set the clan tag for {} bot", name);
    } else if name == "cat_or_not" {
        set_c_char_array(&mut client.clan_tag, ":D");
        set_c_char_array(&mut client.name, "cat_or_nya");

        log::info!("set the clan tag for cat_or_not");
    }
}
