use retour::static_detour;
use rrplug::high::vector::Vector3;
use std::{
    ffi::{c_char, c_void},
    mem,
};

use crate::bindings::{Ray, TraceResults};

static_detour! {
    static TraceLineSimple: unsafe extern "C" fn(*const Vector3, *const Vector3, c_char, c_char, i32, i32, i32, *mut TraceResults);
    static CEngineTraceServer__TraceRayFiltered: unsafe extern "C" fn(*mut c_void, *const Ray, u32, *const c_void, *mut TraceResults);
    static SomeTraceFunction: unsafe extern "C" fn(*mut Ray,usize,i32,u32,c_char, *const TraceResults);
}

#[allow(clippy::too_many_arguments)]
fn hook_trace_line(
    v1: *const Vector3,
    v2: *const Vector3,
    unk1: c_char,
    unk2: c_char,
    unk3: i32,
    unk4: i32,
    unk5: i32,
    trace: *mut TraceResults,
) {
    unsafe {
        log::info!(
            "trace called with v1: {:?}, v2: {:?} unk1: {unk1} unk2: {unk2} unk3: {unk3} unk4: {unk4} unk5: {unk5}",
            *v1,
            *v2
        );
        log::info!("pre trace result : {:?}", *trace);

        TraceLineSimple.call(v1, v2, unk1, unk2, unk3, unk4, unk5, trace);

        log::info!("trace result : {:?}", *trace);
        // dbg!(trace);
    }
}

pub fn hook_server(addr: *const c_void) {
    log::info!("hooking server reversing functions");

    unsafe {
        TraceLineSimple
            .initialize(mem::transmute(addr.offset(0x2725c0)), hook_trace_line)
            .expect("failed to hook TraceLineSimple");
        // .enable()
        // .expect("failure to enable the TraceLineSimple hook");

        log::info!("hooked TraceLineSimple");
    }
}

fn trace_ray_filter_hook(
    this: *mut c_void,
    ray: *const Ray,
    fmask: u32,
    filter: *const c_void,
    trace: *mut TraceResults,
) {
    unsafe {
        log::info!("ray: {:?}", ray.as_ref());
        log::info!("fmask: {:?}", fmask);
    }

    unsafe { CEngineTraceServer__TraceRayFiltered.call(this, ray, fmask, filter, trace) }

    unsafe {
        log::info!("trace: {:?}", trace.as_ref());
    }
}

fn some_trace_function_hook(
    ray: *mut Ray,
    unk1: usize,
    unk2: i32,
    fmask: u32,
    unk3: c_char,
    trace: *const TraceResults,
) {
    unsafe {
        log::info!("ray: {:?}", ray.as_ref());
        log::info!("fmask: {:?}", fmask);
        log::info!("unk1: {:?}", unk1);
        log::info!("unk2: {:?}", unk2);
        log::info!("unk3: {:?}", unk3);
        log::info!("trace: {:?}", trace.as_ref());
    }

    unsafe { SomeTraceFunction.call(ray, unk1, unk2, fmask, unk3, trace) }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking engine reversing functions");

    unsafe {
        CEngineTraceServer__TraceRayFiltered
            .initialize(mem::transmute(addr.offset(0x14eeb0)), trace_ray_filter_hook)
            .expect("failed to hook CEngineTraceServer__TraceRayFiltered");
        // .enable()
        // .expect("failure to enable the CEngineTraceServer__TraceRayFiltered hook");

        log::info!("hooked CEngineTraceServer__TraceRayFiltered");

        SomeTraceFunction
            .initialize(
                mem::transmute(addr.offset(0x1241f0)),
                some_trace_function_hook,
            )
            .expect("failed to hook SomeTraceFunction");
        // .enable()
        // .expect("failure to enable the SomeTraceFunction hook");

        log::info!("hooked SomeTraceFunction");
    }
}
