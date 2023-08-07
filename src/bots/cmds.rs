use rrplug::bindings::entity::SignonState;
use std::ops::Deref;

use super::SIMULATE_CONVAR;
use crate::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    iterate_c_array_sized,
};

pub fn run_bots_cmds() {
    if SIMULATE_CONVAR.wait().get_value_i32() != 1 {
        return;
    }

    let server_functions = SERVER_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let run_null_command = server_functions.run_null_command;

    for (i, client) in unsafe {
        iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()).enumerate()
    } {
        unsafe {
            if *client.signon.deref().deref() == SignonState::FULL && **client.fake_player {
                run_null_command(player_by_index((i + 1).try_into().unwrap()));
            }
        }
    }
}
