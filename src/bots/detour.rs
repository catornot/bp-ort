use super::cmds::run_bots_cmds;
use retour::static_detour;
use rrplug::bindings::entity::CBaseClient;
use std::{
    ffi::{c_char, c_void, CStr},
    mem,
};

static_detour! {
    static SomeRunUsercmdFunc: unsafe extern "C" fn(c_char);
    #[allow(improper_ctypes_definitions)] // this is bad but this is what respawn did with there infinite wisdom
    // static CBaseClient__Connect: unsafe extern "C" fn(CbaseClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256], *const c_void ) -> bool;
    static SomeSubFunc_Connect: unsafe extern "C" fn(*mut CBaseClient, *const c_char);
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

pub fn subfunc_cbaseclient_connect_hook(this: *mut CBaseClient, name: *const c_char) {
    match unsafe { this.as_mut() } {
        Some(client) => {
            if unsafe { *client.fake_player.get_inner() } {
                unsafe {
                    client
                        .clan_tag
                        .iter_mut()
                        .zip(
                            crate::PLUGIN
                                .wait()
                                .bots
                                .clang_tag
                                .lock()
                                .expect("how")
                                .bytes(),
                        )
                        .for_each(|(c, tag_c)| *c = tag_c as i8)
                };

                log::info!("set the clan tag for {}", unsafe {
                    &CStr::from_ptr(client.name.as_ref() as *const [i8] as *const i8)
                        .to_string_lossy()
                });
            }
        }
        None => log::warn!("connected client is null :("),
    }

    unsafe { SomeSubFunc_Connect.call(this, name) }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking engine functions");

    if SomeSubFunc_Connect.is_enabled() {
        return;
    }

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

#[allow(unused)]
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
