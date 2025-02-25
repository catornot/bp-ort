#![allow(unused)]

use libc::{c_char, c_void};
use once_cell::sync::Lazy;
use retour::static_detour;
use rrplug::{
    bindings::squirreldatatypes::HSquirrelVM,
    high::{
        engine::{EngineGlobal, EngineToken},
        UnsafeHandle,
    },
    mid::{squirrel::SQVM_SERVER, utils::str_from_char_ptr},
};
use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    mem,
};

static_detour! {
    static FUN_180723940: unsafe extern "C" fn(*const c_char) -> i32;
    static MaybeRegisterNetVar: unsafe extern "C" fn(*const c_void, i32, usize) -> i32;
    static FUN_180005dd0: unsafe extern "C" fn(*const HSquirrelVM, i32, usize) -> usize;
}

static NEXT_NETVAR_NAME: UnsafeHandle<UnsafeCell<*const c_char>> =
    unsafe { UnsafeHandle::new(UnsafeCell::new(c"uwu".as_ptr())) };
static NEXT_NETVAR_INDEX: UnsafeHandle<UnsafeCell<i32>> =
    unsafe { UnsafeHandle::new(UnsafeCell::new(0)) };
pub static NETVARS: EngineGlobal<RefCell<Lazy<HashMap<String, i32>>>> =
    EngineGlobal::new(RefCell::new(Lazy::new(HashMap::new)));

fn maybe_register_netvar_hook(var: *const c_void, index: i32, unk: usize) -> i32 {
    log::info!("index: {index}");
    unsafe {
        log::info!(
            "name: {:?}; index: {}",
            str_from_char_ptr(*NEXT_NETVAR_NAME.get().get()),
            *NEXT_NETVAR_INDEX.get().get()
        )
    };
    unsafe {
        let sqvm = SQVM_SERVER.get(EngineToken::new_unchecked()).borrow();

        if let Some(sqvm) = sqvm.as_ref().copied() {
            let top = dbg!(sqvm.as_ref()._top);

            log::info!(
                "top: {:?}",
                sqvm.as_ref()
                    ._stack
                    .add(top as usize)
                    .as_ref()
                    .map(|obj| obj._Type)
            );
        }
    }
    let index = unsafe { MaybeRegisterNetVar.call(var, index, unk) };
    log::info!("index: {index}");

    index
}

fn fun_180723940_hook(name: *const c_char) -> i32 {
    unsafe {
        *NEXT_NETVAR_NAME.get().get() = name;
        *NEXT_NETVAR_INDEX.get().get() = FUN_180723940.call(name);
        *NEXT_NETVAR_INDEX.get().get()
    }
}

fn fun_180005dd0_hook(sqvm: *const HSquirrelVM, index: i32, unk: usize) -> usize {
    unsafe {
        log::info!("index: {index}");
        dbg!(FUN_180005dd0.call(sqvm, index, unk))
    }
}
pub fn netvar_hook_server(addr: *const c_void) {
    log::info!("hooking bot server functions");

    unsafe {
        MaybeRegisterNetVar
            .initialize(
                mem::transmute(addr.offset(0x1f9b00)),
                maybe_register_netvar_hook,
            )
            .expect("failed to hook MaybeRegisterNetVar")
            .enable()
            .expect("failure to enable the MaybeRegisterNetVar");

        log::info!("hooked MaybeRegisterNetVar");
        FUN_180723940
            .initialize(mem::transmute(addr.offset(0x723940)), fun_180723940_hook)
            .expect("failed to hook FUN_180723940")
            .enable()
            .expect("failure to enable the FUN_180723940");

        log::info!("hooked FUN_180723940");

        FUN_180005dd0
            .initialize(mem::transmute(addr.offset(0x5dd0)), fun_180005dd0_hook)
            .expect("failed to hook FUN_180005dd0")
            .enable()
            .expect("failure to enable the FUN_180005dd0");

        log::info!("hooked FUN_180005dd0");
    }
}
