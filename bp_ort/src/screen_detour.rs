#![allow(dead_code, unused_variables)]

use retour::static_detour;
use std::{
    ffi::{c_uint, c_void},
    mem,
};

use crate::bindings::MATSYS_FUNCTIONS;

static_detour! {
    static SomeCtextureConstructor: unsafe extern "C" fn(*const c_void ) -> *const c_void;
    static SomeCtextureConsumer: unsafe extern "C" fn(*const c_void) -> *const c_void;
    static MaybeCreate2DTexture: unsafe extern "C" fn(*const c_void, c_uint, *const c_void);
}

fn ctexture_constructor_hook(unk1: *const c_void) -> *const c_void {
    unsafe {
        let ptr = SomeCtextureConstructor.call(unk1);

        log::info!("texture ptr : {ptr:?}");

        let vtable = &*mem::transmute::<_, *const [*const c_void; 5]>(ptr);

        log::info!("vtable {:?}", vtable);

        ptr
    }
}

fn ctexture_consumer_hook(unk1: *const c_void) -> *const c_void {
    unsafe {
        let ptr = SomeCtextureConstructor.call(unk1);

        log::info!("texture ptr : {ptr:?}");

        let vtable = &*mem::transmute::<_, *const [*const c_void; 5]>(ptr);

        log::info!("vtable {:?}", vtable);

        // let get_name: unsafe extern "C" fn(*const c_void) -> *const c_char = mem::transmute(vtable[0]);

        // let name = CStr::from_ptr(get_name(ptr)).to_string_lossy().to_string();

        // log::info!("name {name}");

        // std::ptr::null()

        log::info!(
            "some int {}",
            (MATSYS_FUNCTIONS.wait().some_ctexture_function)(ptr, 10)
        );

        ptr
    }
}

fn create_2d_texture_hook(unk1: *const c_void, unk2: c_uint, unk3: *const c_void) {
    unsafe {
        MaybeCreate2DTexture.call(unk1, unk2, unk3);

        log::info!("maybe texture ptr? : {unk1:?} / {unk3:?}, wtf is this? {unk2}");
    }
}

pub fn hook_materialsystem(addr: *const c_void) {
    // unsafe {
    //     SomeCtextureConstructor
    //         .initialize(
    //             mem::transmute(addr.offset(0x000767a0)),
    //             ctexture_constructor_hook,
    //         )
    //         .expect("failed to hook SomeCtextureConstructor")
    //         .enable()
    //         .expect("failure to enable the SomeCtextureConstructor hook");

    //     log::info!("hooked SomeCtextureConstructor");

    //     SomeCtextureConsumer
    //         .initialize(
    //             mem::transmute(addr.offset(0x000767a0)),
    //             ctexture_consumer_hook,
    //         )
    //         .expect("failed to hook SomeCtextureConsumer")
    //         .enable()
    //         .expect("failure to enable the SomeCtextureConsumer hook");

    //     log::info!("hooked SomeCtextureConsumer");

    //     MaybeCreate2DTexture
    //         .initialize(
    //             mem::transmute(addr.offset(0x00003210)),
    //             create_2d_texture_hook,
    //         )
    //         .expect("failed to hook MaybeCreate2DTexture")
    //         .enable()
    //         .expect("failure to enable the MaybeCreate2DTexture hook");

    //     log::info!("hooked MaybeCreate2DTexture");
    // };
}

// the size of CTexture is 0x90 which is weird? based on this decompile addr 0x000783e2
// seams like ghidra managed to find all the functions in the CTexture vtable
// 0x00003210 is interesting
// oops CTexture is for rpaks, I am looking for vtfs so CVTFTexture
