use retour::static_detour;
use rrplug::{bindings::cvar::command::CCommand, prelude::EngineToken};
use std::{ffi::c_void, mem};

use super::{DRAWWORLD_CONVAR, PAUSABLE_CONVAR};

static_detour! {
    // static R_DrawWorldMeshes: unsafe extern "C" fn(*mut c_void, *mut c_void, u32); // TODO: move this to somewhere else
    static SomeDrawWorldMeshes: unsafe extern "C" fn(*mut c_void, u32, usize); // TODO: move this to somewhere else
    static Host_setpause_f: unsafe extern "C" fn(*mut CCommand); // TODO: move this to somewhere else
}

#[allow(unused)]
pub fn hook_server(addr: *const c_void) {
    log::info!("hooking server dev functions");
}

pub fn some_draw_world_hook(node: *mut c_void, mut unk: u32, unk2: usize) {
    if DRAWWORLD_CONVAR
        .get(unsafe { EngineToken::new_unchecked() })
        .borrow()
        .as_ref()
        .map(|cvar| cvar.get_value_i32() == 0)
        .unwrap_or(false)
    {
        unk = 0;
    }

    unsafe { SomeDrawWorldMeshes.call(node, unk, unk2) }
}

fn set_pause_hook(command: *mut CCommand) {
    if PAUSABLE_CONVAR
        .get(unsafe { EngineToken::new_unchecked() })
        .borrow()
        .as_ref()
        .map(|cvar| cvar.get_value_i32() == 1)
        .unwrap_or(false)
    {
        unsafe { Host_setpause_f.call(command) }
    }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking engine dev functions");

    unsafe {
        SomeDrawWorldMeshes
            .initialize(mem::transmute(addr.offset(0xb8670)), some_draw_world_hook) //0xb7f80
            .expect("failed to hook R_DrawWorldMeshes")
            .enable()
            .expect("failure to enable the R_DrawWorldMeshes hook");

        log::info!("hooked R_DrawWorldMeshes");

        Host_setpause_f
            .initialize(mem::transmute(addr.offset(0x15ccb0)), set_pause_hook) //0xb7f80
            .expect("failed to hook Host_setpause_f")
            .enable()
            .expect("failure to enable the Host_setpause_f hook");

        log::info!("hooked Host_setpause_f");
    }
}
