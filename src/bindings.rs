use rrplug::{
    bindings::entity::{CBaseClient, CBasePlayer},
    engine_functions,
    high::vector::Vector3,
    offset_struct,
};
use std::ffi::{c_char, c_int, c_short, c_uchar, c_void};

pub type PServer = *const c_void;
pub type BotName = *const c_char;
pub type ServerGameClients = *const c_void;
pub type PlayerByIndex = unsafe extern "C" fn(i32) -> *mut CBasePlayer;
pub type ClientFullyConnected = unsafe extern "C" fn(ServerGameClients, u16, bool);
pub type RunNullCommand = unsafe extern "C" fn(*const CBasePlayer);
pub type ProcessUsercmds =
    unsafe extern "C" fn(*mut CBasePlayer, c_short, *const CUserCmd, i32, i32, c_char, c_uchar);
pub type CreateFakeClient = unsafe extern "C" fn(
    PServer,
    BotName,
    *const c_char,
    *const c_char,
    i32,
    i32,
) -> *const CBaseClient;
pub type SomeCtextureFunction = unsafe extern "C" fn(*const c_void, c_int) -> i16;

pub type CreateInterfaceFn =
    unsafe extern "C" fn(name: *const c_char, return_code: *mut c_int) -> *const c_void;

#[repr(C)]
pub struct InterfaceReg {
    pub init: extern "C" fn() -> *const c_void,
    pub name: *const c_char,
    pub next: *const Self,
}

#[allow(clippy::upper_case_acronyms)]
pub type DWORD = ::std::os::raw::c_uint;
#[allow(clippy::upper_case_acronyms)]
pub type BYTE = ::std::os::raw::c_uchar;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct CUserCmd {
    pub command_number: DWORD,
    pub tick_count: DWORD,
    pub command_time: f32,
    pub world_view_angles: Vector3,
    pub gap18: [BYTE; 4usize],
    pub local_view_angles: Vector3,
    pub attackangles: Vector3,
    pub move_: Vector3,
    pub buttons: DWORD,
    pub impulse: BYTE,
    pub weaponselect: ::std::os::raw::c_short,
    pub meleetarget: DWORD,
    pub gap_4c: [BYTE; 24usize],
    pub headoffset: ::std::os::raw::c_char,
    pub gap65: [BYTE; 11usize],
    pub camera_pos: Vector3,
    pub camera_angles: Vector3,
    pub gap88: [BYTE; 4usize],
    pub tick_something: ::std::os::raw::c_int,
    pub dword90: DWORD,
    pub predicted_server_event_hack: DWORD,
    pub dword98: DWORD,
    pub frame_time: f32,
}

offset_struct! {
    pub struct CGlobalVars {
        real_time: f64 where offset(0x0),
        frame_count: i32 where offset(0x8),
        absolute_frame_time: f32 where offset(0xc),
        cur_time: f32 where offset(0x10),
        // there is stuff here too (I skiped things)
        tick_count: u32 where offset(0x3C),
        // there is more but I don't n eed more
    }
}

// struct IServerGameEnts {}

// a really interesting function : FUN_00101370
// it prints CBasePlayer stuff

engine_functions! {
    ENGINE_FUNCTIONS + EngineFunctions for WhichDll::Engine => {
        client_array = *mut CBaseClient, at 0x12A53F90;
        server = PServer, at 0x12A53D40;
        game_clients = ServerGameClients, at 0x13F0AAA8;
        create_fake_client = CreateFakeClient, at 0x114C60;
        globals = *const CGlobalVars, at 0x7C6F70;
    }
}

/*
self.client_fully_connected = unsafe { mem::transmute(handle_server.offset(0x153B70)) };
self.run_null_command = unsafe { mem::transmute(handle_server.offset(0x5A9FD0)) };
self.player_by_index = unsafe { mem::transmute(handle_server.offset(0x26AA10)) };
*/

engine_functions! {
    SERVER_FUNCTIONS + ServerFunctions for WhichDll::Server => {
        base = *const c_void, at 0x0;
        client_fully_connected = ClientFullyConnected, at 0x153B70;
        run_null_command = RunNullCommand, at 0x5A9FD0;
        proccess_user_cmds = ProcessUsercmds, at 0x159e50;
        get_player_by_index = PlayerByIndex, at 0x26AA10;
        interface_regs = *const InterfaceReg, at 0x01752038;
        get_eye_pos = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x0043b8d0;
        get_center_pos = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x00407d30; // found these by pocking around in a vtable :)
        get_angles_01 = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x00442ce0;
        get_angles = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x0043c030;
        get_origin_varient = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x00443e80;
        get_origin = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x004198d0;
        eye_angles = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *const Vector3, at 0x004455f0; // this access the vtable
    }
}

engine_functions! {
    CLIENT_FUNCTIONS + ClientFunctions for WhichDll::Client => {

    }
}

engine_functions! {
    MATSYS_FUNCTIONS + MatSysFunctions for WhichDll::Other("materialsystem_dx11.dll") => {
        some_ctexture_function = SomeCtextureFunction, at 0x00079e80;
    }
}
