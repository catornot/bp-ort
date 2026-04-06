use retour::static_detour;
use rrplug::{bindings::cvar::command::CCommand, prelude::EngineToken};
use std::{ffi::c_void, mem};

use super::{DRAWWORLD_CONVAR, PAUSABLE_CONVAR};

static_detour! {
    // static R_DrawWorldMeshes: unsafe extern "C" fn(*mut c_void, *mut c_void, u32); // TODO: move this to somewhere else
    static SomeDrawWorldMeshes: unsafe extern "C" fn(*mut c_void, u32, usize); // TODO: move this to somewhere else
    static Host_setpause_f: unsafe extern "C" fn(*mut CCommand); // TODO: move this to somewhere else
    // OriginSDK.dll
    static CheckIfOriginIsInstalled: unsafe extern "C" fn() -> bool;
    static TryToStartOrigin: unsafe extern "C" fn(*mut ()) -> u64;
}

#[allow(unused)]
pub fn hook_server(addr: *const c_void) {
    log::info!("hooking server dev functions");
}

pub fn some_draw_world_hook(node: *mut c_void, mut unk: u32, unk2: usize) {
    if DRAWWORLD_CONVAR
        .get(unsafe { EngineToken::new_unchecked() })
        .try_borrow()
        .map(|convar| convar.as_ref().map(|cvar| cvar.get_value_i32() == 0)) // for some reason sometimes the borrow is mut?
        .ok()
        .flatten()
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

fn check_if_origin_is_installed_hook() -> bool {
    if unsafe { CheckIfOriginIsInstalled.call() } {
        log::warn!(
            "Origin is NOT installed according to OriginSDK's check (HKLM\\SOFTWARE\\Wow6432Node\\Origin\\ClientPath is missing/empty)."
        );
        log::warn!(
            "We are bypassing this check in case LSX server is remotely ran (such as in Linux in another WINE prefix), but note that things will fail if Origin is actually not running."
        );
    }
    true
}

fn try_to_start_origin_hook(a1: *mut ()) -> u64 {
    // Calling the original still has it try to start up Origin/EA App if it's down
    // We just want to ignore a failure in case we're on Linux and LSX is started in a different prefix...
    if unsafe { TryToStartOrigin.call(a1) } != 0 {
        log::warn!(
            "Origin process has failed to start. We are ignoring this and let it fail on LSX connection attempt, because LSX might still be up regardless, even if this failed."
        );
    }
    0
}

// thanks p0 https://github.com/p0358/black_market_edition/commit/1c34d06c34266e52d3651ba79d8b733223e4e7fd
pub fn hook_origin_sdk(addr: *const c_void) {
    unsafe {
        CheckIfOriginIsInstalled
            .initialize(
                mem::transmute(addr.offset(0xa1850)),
                check_if_origin_is_installed_hook,
            ) //0xa1850
            .expect("failed to hook CheckIfOriginIsInstalled")
            .enable()
            .expect("failure to enable the CheckIfOriginIsInstalled hook");

        log::info!("hooked CheckIfOriginIsInstalled");

        TryToStartOrigin
            .initialize(
                mem::transmute(addr.offset(0xa19b0)),
                try_to_start_origin_hook,
            ) //0xa19b0
            .expect("failed to hook TryToStartOrigin")
            .enable()
            .expect("failure to enable the TryToStartOrigin hook");

        log::info!("hooked TryToStartOrigin");
    }
}
