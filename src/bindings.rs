// use recastnavigation_sys::dtNavMesh;
use rrplug::{
    bindings::{
        class_types::{c_player::C_Player, client::CClient, cplayer::CPlayer},
        cvar::{
            command::{CCommand, ConCommand, FnCommandCallback_t},
            convar::Color,
        },
        squirreldatatypes::CSquirrelVM,
    },
    high::vector::Vector3,
    offset_functions, offset_struct,
};
use std::{
    ffi::{c_char, c_int, c_short, c_uchar, c_void},
    mem::MaybeUninit,
};

pub type PServer = *const c_void;
pub type BotName = *const c_char;
pub type ServerGameClients = *const c_void;
pub type PlayerByIndex = unsafe extern "C" fn(i32) -> *mut CPlayer;
pub type ClientFullyConnected = unsafe extern "C" fn(ServerGameClients, u16, bool);
pub type RunNullCommand = unsafe extern "C" fn(*const CPlayer);
pub type ProcessUsercmds = unsafe extern "C" fn(
    this: *const ServerGameClients,
    edict: c_short,
    cmds: *const CUserCmd,
    numcmds: i32,
    dropped: i32,
    ignore: c_char,
    paused: c_uchar,
);
pub type CreateFakeClient = unsafe extern "C" fn(
    PServer,
    BotName,
    *const c_char,
    *const c_char,
    i32,
    i32,
) -> *const CClient;
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
#[derive(Debug, Copy, Clone)]
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
    pub gap_a0: [c_char; 152], // eh
}

impl CUserCmd {
    pub fn init_default(sv_funcs: &ServerFunctions) -> Self {
        let mut cmd = MaybeUninit::zeroed();
        unsafe {
            (sv_funcs.create_null_user_cmd)(cmd.as_mut_ptr());
            cmd.assume_init()
        }
    }
}

#[repr(u32)]
#[allow(dead_code)]
pub enum Action {
    Null = 0,
    Attack = 1,
    Jump = 2,
    Duck = 4,
    Forward = 8,
    Back = 0x10,
    Use = 0x20,
    Pausemenu = 0x40,
    Left = 0x80,
    Right = 0x100,
    Moveleft = 0x200,
    Moveright = 0x400,
    Walk = 0x800,
    Reload = 0x1000,
    WeaponDiscard = 0x4000,
    Speed = 0x8000,
    Zoom = 0x10000,
    ZoomToggle = 0x20000,
    Melee = 0x40000,
    WeaponCycle = 0x80000,
    OffHand0 = 0x100000,
    OffHand1 = 0x200000,
    OffHand2 = 0x400000,
    OffHand3 = 0x800000,
    OffHand4 = 0x1000000,
    OffhandQuick = 0x2000000,
    Ducktoggle = 0x4000000,
    UseAndReload = 0x8000000,
    Dodge = 0x10000000,
    VariableScopeToggle = 0x20000000,
    Ping = 0x40000000,
}

offset_struct! {
    pub struct CGlobalVars {
        real_time: f64 where offset(0x0),
        frame_count: i32 where offset(0x8),
        absolute_frame_time: f32 where offset(0xc),
        cur_time: f32 where offset(0x10),
        frametime: f32 where offset(0x30),
        // there is stuff here too (I skiped things)
        tick_count: u32 where offset(0x3C),
        // there is more but I don't n eed more
    }
}

// opaque type
#[repr(C)]
pub struct CBaseEntity;

#[derive(Debug, Clone)]
#[repr(C)]
#[allow(dead_code)]
pub enum CmdSource {
    Code,
    ClientCmd,
    UserInput,
    NetClient,
    NetServer,
    DemoFile,
    Invalid = -1,
}

#[repr(C)]
#[derive(Debug)]
pub struct TraceResults {
    pub gap_0: [c_char; 15],
    pub end_pos: Vector3,
    pub gap_0x1c: [c_char; 4],
    pub surfce_normal: Vector3,
    pub gap_0x2c: [c_char; 4],
    pub fraction: f32,
    pub contents: i32,
    pub field28_0x38: c_uchar,
    pub all_solid: bool,
    pub start_solid: bool,
    pub gap_0x3c: [c_char; 4],
    pub fraction_left_solid: f32,
    pub gap_0x44: [c_char; 15],
    pub hit_sky: bool,
    pub gap_0x55: [c_char; 3],
    pub hit_group: i32,
    pub gap_0x5c: [c_char; 4],
    pub hit_ent: *const CBaseEntity,
    pub gap_0x68: [c_char; 4],
    pub static_prop_index: i32,
}

// struct IServerGameEnts {}

// a really interesting function : FUN_00101370
// it prints CPlayer stuff

offset_functions! {
    ENGINE_FUNCTIONS + EngineFunctions for WhichDll::Engine => {
        client_array = *mut CClient where offset(0x12A53F90);
        host_client = *mut *mut CClient where offset(0x13158990);
        cmd_source = *const isize where offset(0x12A53F90); // when 1 host_client is invalid
        is_dedicated = *const bool where offset(0x13002498);
        server = PServer where offset(0x12A53D40);
        game_clients = ServerGameClients where offset(0x13F0AAA8);
        create_fake_client = CreateFakeClient where offset(0x114C60);
        cclient_disconnect = unsafe extern "C" fn(*mut CClient, u32, *const c_char) where offset(0x1012C0);
        get_current_playlist_var = unsafe extern "C" fn(*const c_char, c_int) -> *const c_char where offset(0x18C680);
        globals = *mut CGlobalVars where offset(0x7C6F70);
        render_line = unsafe extern "C" fn(*const Vector3, *const Vector3, Color, bool) where offset(0x192A70);

        props_and_wolrd_filter = *const c_void where offset(0x5eb980);
        trace_ray = unsafe extern "C" fn(this: *const c_void, ray: *const c_void, maskf: f32, filter: *const c_void, trace: *mut TraceResults ) where offset(0x14eeb0);

        cbuf_add_text = unsafe extern "C" fn(i32, *const c_char, CmdSource) where offset(0x1203B0);
        cbuf_execute = unsafe extern "C" fn() where offset(0x1204B0);
        cbuf_get_current_player = unsafe extern "C" fn() -> i32 where offset(0x120630);
        ccommand_tokenize = unsafe extern "C" fn(*mut CCommand, *const c_char, CmdSource) -> () where offset(0x418380);
        cmd_exec_f = unsafe extern "C" fn(*const CCommand, bool, bool) -> () where offset(0x418380);
        cengine_client_server_cmd = unsafe extern "C" fn(*const c_void, *const c_char, bool) -> () where offset(0x54840);
        cengine_client_client_cmd = unsafe extern "C" fn(*const c_void, *const c_char) -> () where offset(0x4fb50);

        ctraceengine = *const c_void where offset(0x7c9900);
    }
}

offset_functions! {
    SERVER_FUNCTIONS + ServerFunctions for WhichDll::Server => {
        base = *const c_void where offset(0x0);
        move_helper = *mut c_void where offset(0xc389e0);
        csqvm = *mut CSquirrelVM where offset(0xf39358);
        client_fully_connected = ClientFullyConnected where offset(0x153B70);
        run_null_command = RunNullCommand where offset(0x5A9FD0);
        simulate_player = unsafe extern "C" fn(*const CPlayer) where offset(0x0492580);
        proccess_user_cmds = ProcessUsercmds where offset(0x159e50);
        add_user_cmd_to_player = unsafe extern "C" fn(this: *const CPlayer, cmds: *const CUserCmd, numcmds: u32, unk: usize, totalcmds: u32, paused: c_char) where offset(0x005a81c0);
        create_null_user_cmd = unsafe extern "C" fn(*mut CUserCmd) -> *mut CUserCmd where offset(0x25f790);
        player_run_command = unsafe extern "C" fn(*mut CPlayer, *mut CUserCmd,*const c_void) -> () where offset(0x5a7d80);
        set_base_time = unsafe extern "C" fn(*mut CPlayer, f32) where offset(0x5b3790);
        set_last_cmd = unsafe extern "C" fn(*mut CUserCmd, *mut CUserCmd) -> () where offset(0x25f860);
        get_player_by_index = PlayerByIndex where offset(0x26AA10);
        util_get_command_client = unsafe extern "C" fn() -> *mut CPlayer where offset(0x15bf40);
        interface_regs = *const InterfaceReg where offset(0x01752038);
        get_eye_pos = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x0043b8d0);
        get_center_pos = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x00407d30); // found these by pocking around in a vtable :)
        get_angles_01 = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x00442ce0);
        get_angles = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x0043c030);
        get_origin_varient = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x00443e80);
        get_origin = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x004198d0);
        eye_angles = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *const Vector3 where offset(0x004455f0); // this acceses the vtable
        is_on_ground = unsafe extern "C" fn(*const CPlayer) -> usize where offset(0x441c60);
        is_alive = unsafe extern "C" fn(*const CPlayer) -> usize where offset(0x4461e0);
        is_titan = unsafe extern "C" fn(*const CPlayer) -> bool where offset(0x406a70);
        set_health = unsafe extern "C" fn(*mut CPlayer, i32, usize, usize) -> () where offset(0x42d7f0);

        get_offhand_weapon = unsafe extern "C" fn(*const CPlayer,u32 ) -> bool where offset(0xe1ec0); // not done
        set_weapon_by_slot = unsafe extern "C" fn(*const c_void, *const c_char) where offset(0xe4ba0);
        replace_weapon = unsafe extern "C" fn(*const CPlayer, *const c_char, *const c_void, *const c_void) where offset(0xdbae0);
        get_active_weapon = unsafe extern "C" fn(*const CPlayer) -> *const CBaseEntity where offset(0xea4c0);

        trace_line_simple = unsafe extern "C" fn(*const Vector3, *const Vector3, c_char, c_char, i32, i32, i32, *mut TraceResults )  where offset(0x2725c0);

        ent_fire = unsafe extern "C" fn(entityInstance: *mut CBaseEntity, inputName: *const c_char, args: *const c_void, delay: f32, otherEntity: *mut CBaseEntity, unkOrNull: *const c_void, unk:c_char ) where offset(0x29ea70);

        register_con_command = unsafe extern "C" fn(concommand: *mut ConCommand,name: *const c_char, callback: FnCommandCallback_t, helpString: *const c_char,flags: i32, completion: unsafe extern "C-unwind" fn(arg1: *const ::std::os::raw::c_char, arg2: *mut [::std::os::raw::c_char; 128usize]) -> ::std::os::raw::c_int) -> *mut ConCommand where offset(0x723fa0);
        // nav_mesh = *mut *mut dtNavMesh where offset(0x105F5D0);

        get_pet_titan = unsafe extern "C" fn(*const CPlayer) -> *const CBaseEntity where offset(0x5dd940);
    }
}
// very intersting call at server.dll + 0x151782
// call that possibly sets 1 max player for sp? : 0x15191a server.dll

offset_functions! {
    CLIENT_FUNCTIONS + ClientFunctions for WhichDll::Client => {
        get_c_player_by_index = unsafe extern "C" fn(i32) -> *mut C_Player where offset(0x348650);
    }
}

offset_functions! {
    MATSYS_FUNCTIONS + MatSysFunctions for WhichDll::Other("materialsystem_dx11.dll") => {
        some_ctexture_function = SomeCtextureFunction where offset(0x00079e80);
    }
}
