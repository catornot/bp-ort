use super::cmds::run_bots_cmds;
use crate::structs::cbaseclient::{CbaseClient, CbaseClientPtr};
use retour::static_detour;
use std::{
    ffi::{c_char, c_void},
    mem,
};

static_detour! {
    static SomeRunUsercmdFunc: unsafe extern "C" fn(c_char);
    #[allow(improper_ctypes_definitions)] // this is bad but this is what respawn did with there infinite wisdom
    // static CBaseClient__Connect: unsafe extern "C" fn(CbaseClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256], *const c_void ) -> bool;
    static SomeSubFunc_Connect: unsafe extern "C" fn(CbaseClientPtr, *const c_char);
    static SomeVoiceFunc: unsafe extern "C" fn(*const c_void, *const c_void) -> *const c_void;

}

fn some_run_user_cmd_hook(parm: c_char) {
    run_bots_cmds();

    unsafe { SomeRunUsercmdFunc.call(parm) }
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
    }
}

pub fn subfunc_cbaseclient_connect_hook(this: CbaseClientPtr, name: *const c_char) {
    match CbaseClient::new(this) {
        Some(client) => {
            if client.is_fake_player() {
                client.set_clan_tag(
                    crate::PLUGIN
                        .wait()
                        .bots.clang_tag
                        .lock()
                        .expect("how")
                        .to_string(),
                );

                log::info!("set the clan tag for {}", client.get_name());
            } else {
                client.set_clan_tag("GAMING".to_string())
            }
        }
        None => log::warn!("connected client is null :("),
    }

    unsafe { SomeSubFunc_Connect.call(this, name) }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking engine functions");

    unsafe {
        SomeSubFunc_Connect
            .initialize(
                mem::transmute(addr.offset(0x00105ed0)),
                subfunc_cbaseclient_connect_hook, // so since we can't double hook, I found a function that can be hook in CBaseClient__Connect
            )
            .expect("failed to hook SomeSubFunc_Connect")
            .enable()
            .expect("failure to enable the SomeSubFunc_Connect hook");

        log::info!("hooked SomeSubFunc_Connect");
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

#[allow(unused_variables)]
// move this lmao
pub fn hook_client(addr: *const c_void) {
    log::info!("hooking client functions");

    // unsafe {
    //     SomeVoiceFunc
    //         .initialize(
    //             mem::transmute(addr.offset(0x1804a6690)),
    //             some_voice_func_hook, // so since we can't double hook, I found a function that can be hook in CBaseClient__Connect
    //         )
    //         .expect("failed to hook SomeVoiceFunc")
    //         .enable()
    //         .expect("failure to enable the SomeVoiceFunc hook");

    //     log::info!("hooked SomeVoiceFunc");
    // }
}
