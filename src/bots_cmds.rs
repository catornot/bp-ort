use crate::{native_types::SignonState, PLUGIN, SIMULATE_CONVAR};

pub fn run_bots_cmds() {
    if SIMULATE_CONVAR.wait().get_value_i32() != 1 {
        return;
    }

    let mut sed = PLUGIN.wait().source_engine_data.lock().expect("how");

    let player_by_index = sed.player_by_index;
    let run_null_command = sed.run_null_command;

    for (i, client) in (&mut sed.client_array).enumerate() {
        unsafe {
            let sigon = client.get_signon();
            let is_fake_player = client.is_fake_player();
            
            if sigon == SignonState::Full && is_fake_player {
                run_null_command(player_by_index((i + 1).try_into().unwrap()));
            }
        }
    }
}
