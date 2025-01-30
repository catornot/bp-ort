use retour::static_detour;
use rrplug::bindings::cvar::convar::ConVar;
use std::ffi::c_void;

static_detour! {
    static SomeVoiceFunc: unsafe extern "C" fn(*const c_void, *const c_void) -> *const c_void;
    static RegisterConvar2: unsafe extern "C" fn(*mut ConVar, *const i8) -> *mut ConVar;
    static RegisterConvar3: unsafe extern "C" fn(*mut ConVar, *const i8, *const c_void, u32, *const c_void, u8, u32, u8, u32);
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
    unsafe { RegisterConvar2.call(this, name) };

    let this_ref = unsafe { this.as_mut().unwrap() };

    if unsafe { std::ffi::CStr::from_ptr(name.cast()) } == c"cl_fovScale" {
        log::info!("cl_fovscale {}", this_ref.m_bHasMax);
        log::info!("cl_fovscale {}", this_ref.m_bHasMin);
        log::info!("cl_fovscale {}", this_ref.m_fMaxVal);
        log::info!("cl_fovscale {}", this_ref.m_fMaxVal);

        this_ref.m_bHasMax = false;
        this_ref.m_bHasMin = false;
        this_ref.m_fMaxVal = f32::MIN;
        this_ref.m_fMaxVal = f32::MAX;
    }

    this
}

#[allow(clippy::too_many_arguments)]
fn register_convar3(
    this: *mut ConVar,
    name: *const i8,
    unk1: *const c_void,
    unk2: u32,
    unk3: *const c_void,
    unk4: u8,
    unk5: u32,
    unk6: u8,
    unk7: u32,
) {
    unsafe { RegisterConvar3.call(this, name, unk1, unk2, unk3, unk4, unk5, unk6, unk7) };

    let this_ref = unsafe { this.as_mut().unwrap() };

    // if unsafe { std::ffi::CStr::from_ptr(name.cast()) } == c"cl_fovScale" {
    log::info!("cl_fovscale {}", this_ref.m_bHasMax);
    log::info!("cl_fovscale {}", this_ref.m_bHasMin);
    log::info!("cl_fovscale {}", this_ref.m_fMaxVal);
    log::info!("cl_fovscale {}", this_ref.m_fMaxVal);

    this_ref.m_bHasMax = false;
    this_ref.m_bHasMin = false;
    this_ref.m_fMaxVal = f32::MIN;
    this_ref.m_fMaxVal = f32::MAX;
    // }
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

        RegisterConvar3
            .initialize(std::mem::transmute(addr.offset(0x11cf30)), register_convar3)
            .expect("failed to hook RegisterConvar3")
            .enable()
            .expect("failure to enable the RegisterConvar3 hook");

        log::info!("hooked RegisterConvar3");
    }
}
