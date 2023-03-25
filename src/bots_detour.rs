use crate::{bots_cmds::run_bots_cmds, tf2dlls::ServerGameClients};
use retour::static_detour;
use std::{
    ffi::{c_char, c_void},
    mem,
};

static mut LOGGED_STUFF: bool = false;

static_detour! {
    static SomeRunUsercmdFunc: unsafe extern "C" fn(c_char);
    pub static ClientFullyConnected: unsafe extern "C" fn(ServerGameClients, u16, bool);
}

fn some_run_user_cmd_hook(parm: c_char) {
    if !unsafe { LOGGED_STUFF } {
        // crate::PLUGIN
        //     .wait()
        //     .source_engine_data
        //     .lock()
        //     .unwrap()
        //     .client_array
        //     .peak_array();

        unsafe { LOGGED_STUFF = true }
    }

    run_bots_cmds();

    unsafe { SomeRunUsercmdFunc.call(parm) }
}

pub fn client_connected_hook(game_clients: ServerGameClients, edict: u16, unk: bool) {
    let game_clients_known = crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .game_clients;

    log::info!("game_clients_known {game_clients_known:?} ; game_clients {game_clients:?}");
    log::info!("edict {edict}");
    log::info!("unk bool {unk}");

    unsafe { ClientFullyConnected.call(game_clients, edict, unk) };
}

pub fn hook_server(addr: *const c_void) {
    log::info!("hooking server functions");

    unsafe {
        SomeRunUsercmdFunc
            .initialize(
                mem::transmute(addr.offset(0x483A50)),
                some_run_user_cmd_hook,
            )
            .expect("failed to hook SomeRunUsercmdFunc")
            .enable()
            .expect("failure to enable the SomeRunUsercmdFunc hook");

        log::info!("hooked SomeRunUsercmdFunc");

        // ClientFullyConnected
        //     .initialize(mem::transmute(addr.offset(0x153B70)), client_connected_hook)
        //     .expect("failed to hook ClientFullyConnected")
        //     .enable()
        //     .expect("failure to enable the ClientFullyConnected hook");

        // log::info!("hooked ClientFullyConnected");
    }
}
