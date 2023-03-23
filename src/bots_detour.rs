use crate::bots_cmds::run_bots_cmds;
use retour::static_detour;
use std::{ffi::c_char, mem};

static mut LOGGED_STUFF: bool = false;

static_detour! {
    static SomeRunUsercmdFunc: unsafe extern "C" fn(c_char);
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

pub fn hook_server(addr: usize) {
    log::info!("hooking server functions");

    unsafe {
        SomeRunUsercmdFunc
            .initialize(mem::transmute(addr + 0x483A50), some_run_user_cmd_hook)
            .expect("failed to hook SomeRunUsercmdFunc")
            .enable()
            .expect("failure to enable the SomeRunUsercmdFunc hook");
    }

    log::info!("hooked SomeRunUsercmdFunc");
}
