use retour::static_detour;
use rrplug::bindings::cvar::convar::ConVar;
use std::ffi::c_void;

static_detour! {
    static SomeVoiceFunc: unsafe extern "C" fn(*const c_void, *const c_void) -> *const c_void;
    static RegisterConvar2: unsafe extern "C" fn(*mut ConVar, *const i8) -> *mut ConVar;
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
pub fn hook_client(addr: *const c_void) {
    log::info!("hooking random client functions");

    // unsafe {
    //     SomeVoiceFunc
    //         .initialize(
    //             mem::transmute(addr.offset(0x4a6690)),
    //             some_voice_func_hook,
    //         )
    //         .expect("failed to hook SomeVoiceFunc")
    //         .enable()
    //         .expect("failure to enable the SomeVoiceFunc hook");

    //     log::info!("hooked SomeVoiceFunc");
    // }
}

fn register_convar2(this: *mut ConVar, name: *const i8) -> *mut ConVar {
    let this_ref = unsafe { this.as_mut().unwrap() };
    this_ref.m_bHasMax = false;
    this_ref.m_bHasMin = false;
    this_ref.m_fMaxVal = f32::MIN;
    this_ref.m_fMaxVal = f32::MAX;

    unsafe { RegisterConvar2.call(this, name) }
}

#[allow(unused)]
pub fn hook_materialsystem(addr: *const c_void) {
    log::info!("hooking random hook_materialsystem functions");

    unsafe {
        RegisterConvar2
            .initialize(
                std::mem::transmute(addr.offset(0x0011d1d0)),
                register_convar2,
            )
            .expect("failed to hook RegisterConvar2")
            .enable()
            .expect("failure to enable the RegisterConvar2 hook");

        log::info!("hooked RegisterConvar2");
    }
}
