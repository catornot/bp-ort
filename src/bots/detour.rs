use retour::static_detour;
use rrplug::{
    bindings::{
        class_types::{client::CClient, cplayer::CPlayer},
        cvar::command::CCommand,
    },
    high::vector::Vector3,
};
use std::{
    ffi::{c_char, c_short, c_uchar, c_void},
    mem,
};

use super::{
    cmds::{replace_cmd, run_bots_cmds},
    set_on_join::set_stuff_on_join,
    DRAWWORLD_CONVAR,
};
use crate::{
    bindings::{CUserCmd, Ray, TraceResults},
    navmesh::bindings::{dtNavMesh, dtNavMeshQuery, dtPolyRef, dtQueryFilter, dtStatus64},
    utils::from_c_string,
};

static_detour! {
    static Physics_RunThinkFunctions: unsafe extern "C" fn(bool);
    // static CClient__Connect: unsafe extern "C" fn(CClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256] this is a *mut c_char, *const c_void ) -> bool;
    static SomeFuncInConnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void);
    static SomeVoiceFunc: unsafe extern "C" fn(*const c_void, *const c_void) -> *const c_void;
    static PlayerRunCommand: unsafe extern "C" fn(*mut CPlayer, *const CUserCmd, *const c_void);
    static ProcessUsercmds: unsafe extern "C" fn(*mut CPlayer, c_short, *const CUserCmd, i32, i32, c_char, c_uchar); // c_uchar might be wrong since undefined
    static CreateNullUserCmd: unsafe extern "C" fn(*mut CUserCmd) -> *mut CUserCmd;
    static SomeFuncInDisconnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void,c_uchar);
    static CClient__Disconnect: unsafe extern "C" fn(*mut CClient, c_uchar, *const c_void, *const c_void);
    static TraceLineSimple: unsafe extern "C" fn(*const Vector3, *const Vector3, c_char, c_char, i32, i32, i32, *mut TraceResults);
    // static R_DrawWorldMeshes: unsafe extern "C" fn(*mut c_void, *mut c_void, u32); // TODO: move this to somewhere else
    static SomeDrawWorldMeshes: unsafe extern "C" fn(*mut c_void, u32, usize); // TODO: move this to somewhere else
    static Host_setpause_f: unsafe extern "C" fn(*mut CCommand); // TODO: move this to somewhere else
    static dtNavMeshQuery__findNearestPoly: unsafe extern "C" fn( *mut dtNavMeshQuery, *const Vector3, *const Vector3, *const dtQueryFilter, *mut dtPolyRef, *mut Vector3) -> dtStatus64;
    static dtNavMeshQuery__init: unsafe extern "C" fn( *mut dtNavMeshQuery, *const dtNavMesh, i32) -> dtStatus64;
}
// lmao hit the recusion limit
static_detour! {
    static CEngineTraceServer__TraceRayFiltered: unsafe extern "C" fn(*mut c_void, *const Ray, u32, *const c_void, *mut TraceResults);
    static SomeTraceFunction: unsafe extern "C" fn(*mut Ray,usize,i32,u32,c_char, *const TraceResults);
}

fn physics_run_think_functions_hook(paused: bool) {
    run_bots_cmds(paused);

    unsafe { Physics_RunThinkFunctions.call(paused) }
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

fn create_null_cmd_hook(cmd: *mut CUserCmd) -> *mut CUserCmd {
    replace_cmd()
        .map(|new_cmd| {
            unsafe { *cmd = *new_cmd };
            cmd
        })
        .unwrap_or_else(|| unsafe { CreateNullUserCmd.call(cmd) })
}

pub fn hook_server(addr: *const c_void) {
    log::info!("hooking server functions");

    unsafe {
        Physics_RunThinkFunctions
            .initialize(
                mem::transmute(addr.offset(0x483A50)),
                physics_run_think_functions_hook,
            )
            .expect("failed to hook Physics_RunThinkFunctions")
            .enable()
            .expect("failure to enable the Physics_RunThinkFunctions hook");

        log::info!("hooked Physics_RunThinkFunctions");

        TraceLineSimple
            .initialize(mem::transmute(addr.offset(0x2725c0)), hook_trace_line)
            .expect("failed to hook TraceLineSimple");
        // .enable()
        // .expect("failure to enable the TraceLineSimple hook");

        log::info!("hooked TraceLineSimple");

        ProcessUsercmds
            .initialize(
                mem::transmute(addr.offset(0x159e50)),
                hook_proccess_user_cmds,
            )
            .expect("failed to hook ProcessUsercmds");
        // .enable()
        // .expect("failure to enable the ProcessUsercmds hook");

        log::info!("hooked ProcessUsercmds");

        CreateNullUserCmd
            .initialize(mem::transmute(addr.offset(0x25f790)), create_null_cmd_hook)
            .expect("failed to hook CreateNullUserCmd")
            .enable()
            .expect("failure to enable the CreateNullUserCmd hook");

        log::info!("hooked CreateNullUserCmd");
    }
}

pub fn subfunc_cclient_connect_hook(this: *mut CClient, unk1: *const c_void) {
    unsafe { SomeFuncInConnectProcedure.call(this, unk1) }

    if let Some(client) = unsafe { this.as_mut() } {
        unsafe { set_stuff_on_join(client) }
    }
}

pub fn subfunc_cclient_disconnect_hook(this: *mut CClient, unk1: *const c_void, unk2: c_uchar) {
    unsafe { SomeFuncInDisconnectProcedure.call(this, unk1, unk2) }
}

pub fn disconnect_hook(
    this: *mut CClient,
    unk1: c_uchar,
    unk2: *const c_void,
    unk3: *const c_void,
) {
    unsafe { CClient__Disconnect.call(this, unk1, unk2, unk3) }
}

// pub fn draw_world_hook(this: *mut c_void, node: *mut c_void, unk: u32) {
// this broke :(
pub fn some_draw_world_hook(node: *mut c_void, mut unk: u32, unk2: usize) {
    if DRAWWORLD_CONVAR.wait().get_value_i32() == 0 {
        unk = 0;
    }

    unsafe { SomeDrawWorldMeshes.call(node, unk, unk2) }
}

fn set_pause_hook(_command: *mut CCommand) {
    // unsafe { Host_setpause_f.call(command) }
}

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking engine functions");

    if SomeFuncInConnectProcedure.is_enabled() {
        return;
    }

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

        SomeFuncInConnectProcedure
            .initialize(
                mem::transmute(addr.offset(0x106270)),
                subfunc_cclient_connect_hook, // so since we can't double hook, I found a function that can be hook in CClient__Connect
            )
            .expect("failed to hook SomeFuncInConnectProcedure")
            .enable()
            .expect("failure to enable the SomeFuncInConnectProcedure hook");

        log::info!("hooked SomeFuncInConnectProcedure");

        SomeFuncInDisconnectProcedure
            .initialize(
                mem::transmute(addr.offset(0x103810)),
                subfunc_cclient_disconnect_hook,
            )
            .expect("failed to hook SomeFuncInDisconnectProcedure")
            .enable()
            .expect("failure to enable the SomeFuncInDisconnectProcedure hook");

        log::info!("hooked SomeFuncInDisconnectProcedure");

        CClient__Disconnect
            .initialize(mem::transmute(addr.offset(0x1012c0)), disconnect_hook)
            .expect("failed to hook CClient__Disconnect")
            .enable()
            .expect("failure to enable the CClient__Disconnect hook");

        log::info!("hooked CClient__Disconnect");

        SomeDrawWorldMeshes
            .initialize(mem::transmute(addr.offset(0xb8670)), some_draw_world_hook) //0xb7f80
            .expect("failed to hook R_DrawWorldMeshes")
            .enable()
            .expect("failure to enable the R_DrawWorldMeshes hook");

        Host_setpause_f
            .initialize(mem::transmute(addr.offset(0x15ccb0)), set_pause_hook) //0xb7f80
            .expect("failed to hook Host_setpause_f")
            .enable()
            .expect("failure to enable the Host_setpause_fhook");

        log::info!("hooked Host_setpause_f");
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

// cool init funtion may be usful to allow people to join singleplayer
// 0x1145bd
// and this to set singleplayer player cap?
// 0x156c86
