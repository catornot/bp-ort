use retour::static_detour;
use rrplug::bindings::class_types::{client::CClient, cplayer::CPlayer};
use std::{
    ffi::{c_char, c_short, c_uchar, c_void},
    mem,
};

use super::{cmds::run_bots_cmds, set_on_join::set_stuff_on_join};
use crate::{bindings::CUserCmd, utils::from_c_string};

static_detour! {
    static Physics_RunThinkFunctions: unsafe extern "C" fn(c_char);
    #[allow(improper_ctypes_definitions)] // this is bad but this is what respawn did with there infinite wisdom
    // static CClient__Connect: unsafe extern "C" fn(CClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256], *const c_void ) -> bool;
    static SomeFuncInConnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void);
    static SomeVoiceFunc: unsafe extern "C" fn(*const c_void, *const c_void) -> *const c_void;
    static PlayerRunCommand: unsafe extern "C" fn(*mut CPlayer, *const CUserCmd, *const c_void);
    static ProcessUsercmds: unsafe extern "C" fn(*mut CPlayer, c_short, *const CUserCmd, i32, i32, c_char, c_uchar); // c_uchar might be wrong since undefined
    static FUN_658c50: unsafe extern "C" fn(*mut u32, *mut u32) -> *mut u32;
}

fn some_run_user_cmd_hook(parm: c_char) {
    run_bots_cmds();

    unsafe { Physics_RunThinkFunctions.call(parm) }
}

fn hook_proccess_user_cmds(
    // disabled
    this: *mut CPlayer,
    unk1: c_short,
    user_cmds: *const CUserCmd,
    numcmds: i32,
    totalcmds: i32,
    unk2: c_char,
    unk3: c_uchar,
) {
    let name =
        unsafe { from_c_string::<String>(&**(*this).community_name as *const _ as *const i8) };
    log::info!("hook_proccess_user_cmds( this: {name}, unk1: {unk1}, user_cmds: {user_cmds:?}, numcmds: {numcmds}, totalcmds: {totalcmds}, unk2: {unk2}, unk3: {unk3})");

    unsafe { ProcessUsercmds.call(this, unk1, user_cmds, numcmds, totalcmds, unk2, unk3) }
}

// hook this to fix it!
fn hook_658c50(unk1: *mut u32, unk2: *mut u32) -> *mut u32 {
    unsafe {
        *unk1 = *unk2;
        _ = sub_hook_658c50(unk1, unk2);
        unk1
    }
}

fn sub_hook_658c50(unk1: *mut u32, unk2: *mut u32) -> Option<*mut u32> {
    unsafe {
        *unk1 = *unk2;
        *unk1.add(1).as_mut()? = *unk2.add(1).as_ref()?;
        *unk1.add(2).as_mut()? = *unk2.add(2).as_ref()?;
        *unk1.add(3).as_mut()? = *unk2.add(3).as_ref()?;
        *unk1.add(4).as_mut()? = *unk2.add(4).as_ref()?;
        *unk1.add(5).as_mut()? = *unk2.add(5).as_ref()?;
        *unk1.add(6).as_mut()? = *unk2.add(6).as_ref()?;
        *unk1.add(7).as_mut()? = *unk2.add(7).as_ref()?;
        *unk1.add(8).as_mut()? = *unk2.add(8).as_ref()?;
        *unk1.add(9).as_mut()? = *unk2.add(9).as_ref()?;
        *unk1.add(10).as_mut()? = *unk2.add(10).as_ref()?;
        *unk1.add(0xb).as_mut()? = *unk2.add(0xb).as_ref()?;
        *unk1.add(0xc).as_mut()? = *unk2.add(0xc).as_ref()?;
        *unk1.add(0xd).as_mut()? = *unk2.add(0xd).as_ref()?;
        *unk1.add(0xe).as_mut()? = *unk2.add(0xe).as_ref()?;
        *unk1.add(0xf).as_mut()? = *unk2.add(0xf).as_ref()?;
        *unk1.add(0x10).as_mut()? = *unk2.add(0x10).as_ref()?;
        *unk1.add(0x11).as_mut()? = *unk2.add(0x11).as_ref()?;
    }
    Some(unk1)
}

pub fn hook_server(addr: *const c_void) {
    log::info!("hooking server functions");

    unsafe {
        Physics_RunThinkFunctions
            .initialize(
                mem::transmute(addr.offset(0x483A50)),
                some_run_user_cmd_hook,
            )
            .expect("failed to hook Physics_RunThinkFunctions")
            .enable()
            .expect("failure to enable the Physics_RunThinkFunctions hook");

        log::info!("hooked Physics_RunThinkFunctions");

        ProcessUsercmds
            .initialize(
                mem::transmute(addr.offset(0x159e50)),
                hook_proccess_user_cmds,
            )
            .expect("failed to hook ProcessUsercmds");
        // .enable()
        // .expect("failure to enable the ProcessUsercmds hook");

        log::info!("hooked ProcessUsercmds");

        FUN_658c50
            .initialize(mem::transmute(addr.offset(0x658c50)), hook_658c50)
            .expect("failed to hook FUN_658c50");
            // .enable()
            // .expect("failure to enable the FUN_658c50 hook");

        log::info!("hooked FUN_658c50");
    }
}

pub fn subfunc_cclient_connect_hook(this: *mut CClient, unk1: *const c_void) {
    unsafe { SomeFuncInConnectProcedure.call(this, unk1) }

    if let Some(client) = unsafe { this.as_mut() } {
        unsafe { set_stuff_on_join(client) }
    }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking engine functions");

    if SomeFuncInConnectProcedure.is_enabled() {
        return;
    }

    unsafe {
        SomeFuncInConnectProcedure
            .initialize(
                mem::transmute(addr.offset(0x00106270)),
                subfunc_cclient_connect_hook, // so since we can't double hook, I found a function that can be hook in CClient__Connect
            )
            .expect("failed to hook SomeFuncInConnectProcedure")
            .enable()
            .expect("failure to enable the SomeFuncInConnectProcedure hook");

        log::info!("hooked SomeFuncInConnectProcedure");
    }
}

// SomeVoiceFunc
#[allow(dead_code)]
fn some_voice_func_hook(unk1: *const c_void, unk2: *const c_void) -> *const c_void {
    unsafe {
        let ptr = SomeVoiceFunc.call(unk1, unk2);

        log::info!("SomeVoicePtr {ptr:?}");

        ptr
    }
}

#[allow(unused)]
// move this lmao
pub fn hook_client(addr: *const c_void) {
    log::info!("hooking client functions");

    // unsafe {
    //     SomeVoiceFunc
    //         .initialize(
    //             mem::transmute(addr.offset(0x1804a6690)),
    //             some_voice_func_hook,
    //         )
    //         .expect("failed to hook SomeVoiceFunc")
    //         .enable()
    //         .expect("failure to enable the SomeVoiceFunc hook");

    //     log::info!("hooked SomeVoiceFunc");
    // }
}
