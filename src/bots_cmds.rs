use std::ffi::{c_char, CStr};

use crate::{native_types::SignonState, PLUGIN, SIMULATE_CONVAR};

pub fn run_bots_cmds() {
    if SIMULATE_CONVAR.wait().get_value_i32() != 1 {
        return;
    }

    let mut sed = PLUGIN.wait().source_engine_data.lock().expect("how");

    let player_by_index = sed.player_by_index;
    let run_null_command = sed.run_null_command;

    for (i, client) in (&mut sed.client_array).map(|c| c as usize).enumerate() {
        unsafe {
            let sigon = SignonState::from(*((client + 0x2A0) as *const i32));
            let is_fake_player = *((client + 0x484) as *const bool);
            let c_name = (client + 0x16) as *const c_char;
            let name = CStr::from_ptr(c_name).to_string_lossy().to_string();

            if is_fake_player {
                log::info!("{name} {sigon:?}")
            }

            if sigon == SignonState::Full && is_fake_player {
                log::info!("running cmds for {name}");
                run_null_command(player_by_index((i + 1).try_into().unwrap()));
            }
        }
    }
}
