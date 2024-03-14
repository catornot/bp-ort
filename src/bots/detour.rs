use retour::static_detour;
use rrplug::bindings::class_types::client::CClient;
use std::{
    ffi::{c_uchar, c_void},
    mem,
};

use super::{
    cmds::{replace_cmd, run_bots_cmds},
    set_on_join::set_stuff_on_join,
};
use crate::bindings::CUserCmd;

static_detour! {
    static Physics_RunThinkFunctions: unsafe extern "C" fn(bool);
    // static CClient__Connect: unsafe extern "C" fn(CClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256] this is a *mut c_char, *const c_void ) -> bool;
    static SomeFuncInConnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void);
    static CreateNullUserCmd: unsafe extern "C" fn(*mut CUserCmd) -> *mut CUserCmd;
    static SomeFuncInDisconnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void,c_uchar);
    static CClient__Disconnect: unsafe extern "C" fn(*mut CClient, c_uchar, *const c_void, *const c_void);
}

fn physics_run_think_functions_hook(paused: bool) {
    run_bots_cmds(paused);

    unsafe { Physics_RunThinkFunctions.call(paused) }
}

fn create_null_cmd_hook(cmd: *mut CUserCmd) -> *mut CUserCmd {
    replace_cmd()
        .map(|new_cmd| {
            unsafe { *cmd = *new_cmd };
            cmd
        })
        .unwrap_or_else(|| unsafe { CreateNullUserCmd.call(cmd) })
}

pub fn hook_server(addr: *const c_void) {
    log::info!("hooking bot server functions");

    unsafe {
        Physics_RunThinkFunctions
            .initialize(
                mem::transmute(addr.offset(0x483A50)),
                physics_run_think_functions_hook,
            )
            .expect("failed to hook Physics_RunThinkFunctions")
            .enable()
            .expect("failure to enable the Physics_RunThinkFunctions hook");

        log::info!("hooked Physics_RunThinkFunctions");

        CreateNullUserCmd
            .initialize(mem::transmute(addr.offset(0x25f790)), create_null_cmd_hook)
            .expect("failed to hook CreateNullUserCmd")
            .enable()
            .expect("failure to enable the CreateNullUserCmd hook");

        log::info!("hooked CreateNullUserCmd");
    }
}

pub fn subfunc_cclient_connect_hook(this: *mut CClient, unk1: *const c_void) {
    unsafe { SomeFuncInConnectProcedure.call(this, unk1) }

    if let Some(client) = unsafe { this.as_mut() } {
        unsafe { set_stuff_on_join(client) }
    }
}

pub fn subfunc_cclient_disconnect_hook(this: *mut CClient, unk1: *const c_void, unk2: c_uchar) {
    unsafe { SomeFuncInDisconnectProcedure.call(this, unk1, unk2) }
}

pub fn disconnect_hook(
    this: *mut CClient,
    unk1: c_uchar,
    unk2: *const c_void,
    unk3: *const c_void,
) {
    unsafe { CClient__Disconnect.call(this, unk1, unk2, unk3) }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking bot engine functions");

    if SomeFuncInConnectProcedure.is_enabled() {
        return;
    }

    unsafe {
        SomeFuncInConnectProcedure
            .initialize(
                mem::transmute(addr.offset(0x106270)),
                subfunc_cclient_connect_hook, // so since we can't double hook, I found a function that can be hook in CClient__Connect
            )
            .expect("failed to hook SomeFuncInConnectProcedure")
            .enable()
            .expect("failure to enable the SomeFuncInConnectProcedure hook");

        log::info!("hooked SomeFuncInConnectProcedure");

        SomeFuncInDisconnectProcedure
            .initialize(
                mem::transmute(addr.offset(0x103810)),
                subfunc_cclient_disconnect_hook,
            )
            .expect("failed to hook SomeFuncInDisconnectProcedure")
            .enable()
            .expect("failure to enable the SomeFuncInDisconnectProcedure hook");

        log::info!("hooked SomeFuncInDisconnectProcedure");

        CClient__Disconnect
            .initialize(mem::transmute(addr.offset(0x1012c0)), disconnect_hook)
            .expect("failed to hook CClient__Disconnect")
            .enable()
            .expect("failure to enable the CClient__Disconnect hook");

        log::info!("hooked CClient__Disconnect");
    }
}

// cool init funtion may be usful to allow people to join singleplayer
// 0x1145bd
// and this to set singleplayer player cap?
// 0x156c86
