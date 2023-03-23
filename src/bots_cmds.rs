use std::{
    ffi::{c_char, CStr},
    mem,
};

use crate::{tf2dlls::CbasePlayer, PLUGIN, SIMULATE_CONVAR};

#[allow(dead_code)]
#[repr(i32)]
#[derive(Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum SignonState {
    #[default]
    None = 0, // no state yet; about to connect
    Challenge = 1,   // client challenging server; all OOB packets
    Connected = 2,   // client is connected to server; netchans ready
    New = 3,         // just got serverinfo and string tables
    Prespawn = 4,    // received signon buffers
    Gettingdata = 5, // respawn-defined signonstate, assumedly this is for persistence
    Spawn = 6,       // ready to receive entity packets
    Firstsnap = 7,   // another respawn-defined one
    Full = 8,        // we are fully connected; first non-delta packet received
    Changelevel = 9, // server is changing level; please wait
}

pub fn run_bots_cmds() {
    if SIMULATE_CONVAR.wait().get_value_i32() != 1 {
        return;
    }

    let sed = PLUGIN.wait().source_engine_data.lock().expect("how");

    for i in 0..32 {
        unsafe {
            let client = sed.client_array as usize + (mem::size_of::<CbasePlayer>() * i);

            // 0x2A0 is m_Signon
            // 0x16 is m_Name
            // 0x484 is m_bFakePlayer

            let sigon = mem::transmute::<_, SignonState>(*((client + 0x2A0) as *const i32));
            let is_fake_player = *((client + 0x484) as *const bool);
            let c_name = (client + 0x484) as *const c_char;
            let name = CStr::from_ptr(c_name).to_string_lossy().to_string();

            if sigon == SignonState::Full && is_fake_player {
                log::info!("running cmds for {name}");
                (sed.run_null_command)((sed.player_by_index)((i + 1).try_into().unwrap()));
            }
        }
    }
}
