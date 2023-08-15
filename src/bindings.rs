use rrplug::{
    bindings::entity::{CBaseClient, CBasePlayer},
    engine_functions,
    high::vector::{QAngle, Vector3},
};
use std::ffi::{c_char, c_int, c_short, c_void};

pub type PServer = *const c_void;
pub type BotName = *const c_char;
pub type ServerGameClients = *const c_void;
pub type PlayerByIndex = unsafe extern "fastcall" fn(i32) -> *mut CBasePlayer;
pub type ClientFullyConnected = unsafe extern "fastcall" fn(ServerGameClients, u16, bool);
pub type RunNullCommand = unsafe extern "fastcall" fn(*const CBasePlayer);
pub type CreateFakeClient = unsafe extern "fastcall" fn(
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

#[repr(C)]
pub struct CUserCmd {
    command_number: c_int,
    tick_count: c_int,
    viewangles: QAngle,
    forwardmove: f32,
    sidemove: f32,
    upmove: f32,
    buttons: c_int,
    impulse: i8, // byte
    weaponselect: c_int,
    weaponsubtype: c_int,
    random_seed: c_int,
    mousedx: c_short,
    mousedy: c_short,
    hasbeenpredicted: bool,
}

// struct IServerGameEnts {}

engine_functions! {
    ENGINE_FUNCTIONS + EngineFunctions for WhichDll::Engine => {
        client_array = *mut CBaseClient, at 0x12A53F90;
        server = PServer, at 0x12A53D40;
        game_clients = ServerGameClients, at 0x13F0AAA8;
        create_fake_client = CreateFakeClient, at 0x114C60;
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
        get_player_by_index = PlayerByIndex, at 0x26AA10;
        interface_regs = *const InterfaceReg, at 0x01752038;
        get_eye_pos = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x0043b8d0;
        get_center_pos = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x00407d30; // found these by pocking around in a vtable :)
        get_angles_01 = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x00442ce0;
        get_angles = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x0043c030;
        get_origin_varient = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x00443e80;
        get_origin = unsafe extern "C" fn(*const CBasePlayer, *mut Vector3) -> *mut Vector3, at 0x004198d0;
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
