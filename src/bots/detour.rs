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
    mem::{self, MaybeUninit},
};

use super::{cmds::run_bots_cmds, set_on_join::set_stuff_on_join, DRAWWORLD_CONVAR};
use crate::{
    bindings::{CUserCmd, TraceResults},
    navmesh::bindings::{dtNavMesh, dtNavMeshQuery, dtPolyRef, dtQueryFilter, dtStatus64},
    utils::from_c_string,
};

static_detour! {
    static Physics_RunThinkFunctions: unsafe extern "C" fn(c_char);
    // static CClient__Connect: unsafe extern "C" fn(CClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256] this is a *mut c_char, *const c_void ) -> bool;
    static SomeFuncInConnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void);
    static SomeVoiceFunc: unsafe extern "C" fn(*const c_void, *const c_void) -> *const c_void;
    static PlayerRunCommand: unsafe extern "C" fn(*mut CPlayer, *const CUserCmd, *const c_void);
    static ProcessUsercmds: unsafe extern "C" fn(*mut CPlayer, c_short, *const CUserCmd, i32, i32, c_char, c_uchar); // c_uchar might be wrong since undefined
    static SomeFuncInDisconnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void,c_uchar);
    static CClient__Disconnect: unsafe extern "C" fn(*mut CClient, c_uchar, *const c_void, *const c_void);
    static TraceLineSimple: unsafe extern "C" fn(*const Vector3, *const Vector3, c_char, c_char, i32, i32, i32, *mut TraceResults);
    // static R_DrawWorldMeshes: unsafe extern "C" fn(*mut c_void, *mut c_void, u32); // TODO: move this to somewhere else
    static SomeDrawWorldMeshes: unsafe extern "C" fn(*mut c_void, u32, usize); // TODO: move this to somewhere else
    static Host_setpause_f: unsafe extern "C" fn(*mut CCommand); // TODO: move this to somewhere else
    static dtNavMeshQuery__findNearestPoly: unsafe extern "C" fn( *mut dtNavMeshQuery, *const Vector3, *const Vector3, *const dtQueryFilter, *mut dtPolyRef, *mut Vector3) -> dtStatus64;
    static dtNavMeshQuery__init: unsafe extern "C" fn( *mut dtNavMeshQuery, *const dtNavMesh, i32) -> dtStatus64;
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

fn find_nearest_poly_hook(
    this: *mut dtNavMeshQuery,
    center: *const Vector3,
    half_extents: *const Vector3,
    filter: *const dtQueryFilter,
    nearest_ref: *mut dtPolyRef,
    nearest_pt: *mut Vector3,
) -> dtStatus64 {
    log::info!("find_nearest_poly called");
    unsafe {
        log::info!("this {:?}", this.as_ref());
        log::info!(
            "open list {:?}",
            this.as_ref().map(|this| this.m_openList.as_ref())
        );
        log::info!("center {:?}", center.as_ref());
        log::info!("half_extents {:?}", half_extents.as_ref());
        log::info!("filter {:?}", filter.as_ref());
    }

    let status = unsafe {
        dtNavMeshQuery__findNearestPoly.call(
            this,
            center,
            half_extents,
            filter,
            nearest_ref,
            nearest_pt,
        )
    };

    unsafe {
        log::info!("nearest_ref {:?}", nearest_ref.as_ref());
        log::info!("nearest_pt {:?}", nearest_pt.as_ref());
    }
    log::info!("status {:X}", status);

    status
}

fn query_init_hook(this: *mut dtNavMeshQuery, nav: *const dtNavMesh, maxnodes: i32) -> dtStatus64 {
    log::info!("query_init_hook called");
    unsafe {
        log::info!("this {:?}", this.as_ref());
        log::info!("query_init_hook {:?}", nav.as_ref());
        log::info!("half_extents {}", maxnodes);
    }

    let status = unsafe { dtNavMeshQuery__init.call(this, nav, maxnodes) };

    unsafe {
        log::info!("this {:?}", this.as_ref());
    }
    log::info!("status 0x{:X}", status);

    if this.is_null() || nav.is_null() {
        return status;
    }

    // unsafe { this.as_mut().unwrap().m_openList = std::mem::transmute(0x449f45ed44cab615usize) };

    let filter: dtQueryFilter = unsafe { MaybeUninit::zeroed().assume_init() };

    const GOAL: Vector3 = Vector3::new(-207.0, -1750.0, 1.0);
    const EXTENTS: Vector3 = Vector3::new(80.0, 80.0, 36.0);

    let mut _ref = 0;
    let mut goal_pos = Vector3::ZERO;
    find_nearest_poly_hook(this, &GOAL, &EXTENTS, &filter, &mut _ref, &mut goal_pos);

    status
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

        dtNavMeshQuery__findNearestPoly
            .initialize(
                mem::transmute(addr.offset(0x3ebe50)),
                find_nearest_poly_hook,
            ) //0xb7f80
            .expect("failed to hook dtNavMeshQuery__findNearestPoly")
            .enable()
            .expect("failure to enable the dtNavMeshQuery__findNearestPoly");

        log::info!("hooked dtNavMeshQuery__findNearestPoly");

        dtNavMeshQuery__init
            .initialize(mem::transmute(addr.offset(0x3f0980)), query_init_hook) //0xb7f80
            .expect("failed to hook dtNavMeshQuery__init")
            .enable()
            .expect("failure to enable the dtNavMeshQuery__init");

        log::info!("hooked dtNavMeshQuery__init");
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
