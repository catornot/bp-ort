use crate::utils::{from_c_string, set_c_char_array};
use rrplug::bindings::class_types::client::CClient;

pub unsafe fn set_stuff_on_join(client: &mut CClient) {
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
    } else {
        log::info!("set the clan tag for {name}");

        let new_name = name.replace(['r', 'l'], "w").replace(['R', 'L'], "W");

        set_c_char_array(&mut client.clan_tag, ":o");
        set_c_char_array(&mut client.name, &new_name);
    }
}
