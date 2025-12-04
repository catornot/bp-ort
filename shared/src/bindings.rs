use rrplug::{
    bindings::{
        class_types::{
            c_player::C_Player, cbaseentity::CBaseEntity, client::CClient, cplayer::CPlayer,
            cplayerdecoy::CPlayerDecoy, globalvars::CGlobalVars,
        },
        cvar::{
            command::{CCommand, ConCommand, FnCommandCallback_t},
            convar::Color,
        },
        squirrelclasstypes::SQRESULT,
        squirreldatatypes::{CSquirrelVM, HSquirrelVM, SQObject},
    },
    high::vector::Vector3,
    offset_functions,
};
use std::{
    ffi::{c_char, c_int, c_short, c_uchar, c_void},
    mem::MaybeUninit,
};

use crate::interfaces::CNetworkStringTable;

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
    *const CServer,
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
pub enum TraceCollisionGroup {
    None = 0,
    Debris = 1,
    DebrisTrigger = 2,
    Player = 5,
    BreakableGlass = 6,
    NPC = 8,
    Weapon = 12,
    Projectile = 14,
    BlockWeapons = 18,
    BlockWeaponsAndPhysics = 19,
}

#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
#[repr(C)]
pub enum Contents {
    // r1/scripts/vscripts/_consts.nut:1159
    EMPTY = 0x00,
    SOLID = 0x01,
    WINDOW = 0x02, // bulletproof glass etc. (transparent but solid)
    AUX = 0x04,    // unused ?
    GRATE = 0x08,  // allows bullets & vis
    SLIME = 0x10,
    WATER = 0x20,
    WINDOW_NO_COLLIDE = 0x40,
    ISOPAQUE = 0x80,         // blocks AI Line Of Sight, may be non - solid
    TEST_FOG_VOLUME = 0x100, // cannot be seen through, but may be non - solid
    UNUSED_1 = 0x200,
    BLOCK_LIGHT = 0x400,
    TEAM_1 = 0x800,
    TEAM_2 = 0x1000,
    IGNORE_NODRAW_OPAQUE = 0x2000, // ignore opaque if Surface.NO_DRAW
    MOVEABLE = 0x4000,
    PLAYER_CLIP = 0x10000, // blocks human players
    MONSTER_CLIP = 0x20000,
    BRUSH_PAINT = 0x40000,
    BLOCK_LOS = 0x80000, // block AI line of sight
    NO_CLIMB = 0x100000,
    TITAN_CLIP = 0x200000, // blocks titan players
    BULLET_CLIP = 0x400000,
    UNUSED_5 = 0x800000,
    ORIGIN = 0x1000000,  // removed before bsping an entity
    MONSTER = 0x2000000, // should never be on a brush, only in game
    DEBRIS = 0x4000000,
    DETAIL = 0x8000000,       // brushes to be added after vis leafs
    TRANSLUCENT = 0x10000000, // auto set if any surface has trans
    LADDER = 0x20000000,
    HITBOX = 0x40000000, // use accurate hitboxes on trace
}

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
#[repr(align(4))]
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

#[repr(C)]
#[derive(Debug)]
pub struct CMoveHelperServer {
    pub vtable: *const c_void,
    pub host: *const CPlayer,
    pub touchlist: *const c_void,
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
// #[repr(align(4))]
#[derive(Debug)]
pub struct CGameTrace {
    pub start_pos: Vector3,
    pub unk1: f32,
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

#[repr(C)]
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HostState {
    NewGame = 0,
    LoadGame,
    ChangeLevelSp,
    ChangeLevelMp,
    Run,
    GameShutdown,
    Shutdown,
    Restart,
}

#[repr(C)]
pub struct CHostState {
    pub current_state: HostState,
    pub next_state: HostState,
    pub vec_location: [i32; 3],
    pub ang_location: [i32; 3],
    pub level_name: [c_char; 32],
    pub map_group_name: [c_char; 32],
    pub landmark_name: [c_char; 32],
    pub save_name: [c_char; 32],
    pub short_frame_time: i32, // run a few one-tick frames to avoid large timesteps while loading assets
    pub active_game: bool,
    pub remember_location: bool,
    pub background_level: bool,
    pub waiting_for_connection: bool,
    pub let_tools_override_load_game_ents: bool, // During a load game, this tells Foundry to override ents that are selected in Hammer.
    pub split_screen_connect: bool,
    pub game_has_shut_down_and_flushed_memory: bool, // This is false once we load a map into memory, and set to true once the map is unloaded
    pub workshop_map_download_pending: bool,
}

#[repr(C)]
#[repr(align(16))]
#[derive(Debug, Copy, Clone)]
pub struct VectorAligned {
    pub vec: Vector3,
    pub w: f32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct Ray {
    pub start: VectorAligned,
    pub delta: VectorAligned,
    pub offset: VectorAligned,
    pub unk3: f32,
    pub unk4: ::std::os::raw::c_longlong,
    pub unk5: f32,
    pub unk6: ::std::os::raw::c_longlong,
    pub unk7: f32,
    pub is_ray: bool,
    pub is_swept: bool,
    pub is_smth: bool,
    pub flags: ::std::os::raw::c_int,
    pub unk8: ::std::os::raw::c_int, // this is just because there seams to be extra fields
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CTraceFilterSimple {
    pub vtable: *const fn(),
    pub unk: i32,
    pub pass_ent: *const CBaseEntity,
    pub should_hit_func: *const (),
    pub collision_group: i32,
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Debug, Clone)]
pub struct CTraceFilterWorldAndProps {
    pub vtable: *const fn(),
    pub pass_ent: *const CBaseEntity,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CEntInfo {
    pub vtable_maybe: *const fn(),
    pub ent: *const CBaseEntity,
    pub serial_number: i32,
    gap: [c_char; 28],
}

#[repr(C)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum server_state_t {
    ss_dead = 0, // Dead
    ss_loading,  // Spawning
    ss_active,   // Running
    ss_paused,   // Running, but paused
}

#[repr(C)]
#[allow(non_snake_case)]
// incomplete type
pub struct CServer {
    pub vtfable: *const c_void,
    pub m_State: server_state_t,
    pub m_Socket: i32,
    pub m_nTickCount: i32,
    pub m_bResetMaxTeams: bool,
    pub m_szMapName: [c_char; 64],
    pub m_szMapGroupName: [c_char; 64],
    pub m_szPassword: [c_char; 32],
    pub worldmapCRC: u32,
    pub clientDllCRC: u32,
    pub unkData: *mut c_void,
    pub m_StringTables: *const c_void, /*CNetworkStringTableContainer*/
    pub m_pInstanceBaselineTable: *const c_void, /*CNetworkStringTable*/
    pub m_pLightStyleTable: *const c_void, /*CNetworkStringTable*/
    pub m_pUserInfoTable: *const c_void, /*CNetworkStringTable*/
    pub m_pServerQueryTable: *const c_void, /*CNetworkStringTable*/
    pub m_bReplay: bool,
    pub m_bUpdateFrame: bool,
    pub m_bUseReputation: bool,
    pub m_bSimulating: bool,
    pub m_nPad: u32,
    pub m_Signon: [c_char; 0x48], // this may be way off or not!
    pub m_SignonBuffer: CUtlMemory<i8>,
    pub m_nServerClasses: i32,
    pub m_nServerClassBits: i32,
    pub m_szConDetails: [c_char; 64],
    pub m_szHostInfo: [c_char; 28],
    pub pad: [c_char; 46],
    pub m_Clients: [CClient; 32],
}

#[repr(C)]
pub struct CUtlMemory<T: ?Sized> {
    pub memory: *mut T,
    pub allocation_count: usize,
    pub grow_size: usize,
}

// illegal const usage for now
// #[repr(C)]
// struct CUtlMemoryFixed<const SIZE: usize, const nAlignment: usize = 0, T: ?Sized> {
//     pub memory: [c_char; SIZE * std::mem::size_of::<T>() + nAlignment],
// }

// struct IServerGameEnts {}

// a really interesting function : FUN_00101370
// it prints CPlayer stuff

offset_functions! {
    ENGINE_FUNCTIONS + EngineFunctions for WhichDll::Engine => {
        base = *const c_void where offset(0x0);
        client_array = *mut CClient where offset(0x12A53F90);
        host_client = *mut *mut CClient where offset(0x13158990);
        cmd_source = *const isize where offset(0x12A53F90); // when 1 host_client is invalid
        is_dedicated = *const bool where offset(0x13002498);
        server = *mut CServer where offset(0x12A53D40);
        game_clients = ServerGameClients where offset(0x13F0AAA8);
        create_fake_client = CreateFakeClient where offset(0x114C60);
        cclient_disconnect = unsafe extern "C" fn(*mut CClient, u32, *const c_char) where offset(0x1012C0);
        get_current_playlist_var = unsafe extern "C" fn(*const c_char, c_int) -> *const c_char where offset(0x18C680);
        globals = *mut CGlobalVars where offset(0x7C6F70);
        render_line = unsafe extern "C" fn(*const Vector3, *const Vector3, Color, bool) where offset(0x192A70);
        cgame_client_printf = unsafe extern "C" fn(client: *const CClient, msg: *const c_char) where offset(0x1016A0);
        cnetwork_string_table_vtable = *const () where offset(0x60FAE8);
        cclient_setname = unsafe extern "C" fn(client: *mut CClient, name: *const c_char) where offset(0x105ed0);

        props_and_world_filter = *const fn() where offset(0x5eb980);
        hit_all_filter = *const fn() where offset(0x5fc520);

        cbuf_add_text = unsafe extern "C" fn(i32, *const c_char, CmdSource) where offset(0x1203B0);
        cbuf_execute = unsafe extern "C" fn() where offset(0x1204B0);
        cbuf_get_current_player = unsafe extern "C" fn() -> i32 where offset(0x120630);
        ccommand_tokenize = unsafe extern "C" fn(*mut CCommand, *const c_char, CmdSource) -> () where offset(0x418380);
        cmd_exec_f = unsafe extern "C" fn(*const CCommand, bool, bool) -> () where offset(0x418380);
        cengine_client_server_cmd = unsafe extern "C" fn(*const c_void, *const c_char, bool) -> () where offset(0x54840);
        cengine_client_client_cmd = unsafe extern "C" fn(*const c_void, *const c_char) -> () where offset(0x4fb50);
        host_state = *mut CHostState where offset(0x7CF180);

        trace_ray_filter = unsafe extern "fastcall-unwind" fn(this: *const c_void, ray: *const Ray, maskf: u32, filter: *const c_void, trace: *mut CGameTrace ) where offset(0x14eeb0);
        trace_ray = unsafe extern "fastcall-unwind" fn(this: *const c_void, ray: *const Ray, maskf: u32, trace: *mut CGameTrace ) where offset(0x14f7a0);
    }
}

offset_functions! {
    SERVER_FUNCTIONS + ServerFunctions for WhichDll::Server => {
        base = *const c_void where offset(0x0);
        move_helper = *mut CMoveHelperServer where offset(0xc389e0);
        csqvm = *mut CSquirrelVM where offset(0xf39358);
        client_fully_connected = ClientFullyConnected where offset(0x153B70);
        run_null_command = RunNullCommand where offset(0x5A9FD0);
        simulate_player = unsafe extern "C" fn(*const CPlayer) where offset(0x0492580);
        proccess_user_cmds = ProcessUsercmds where offset(0x159e50);
        player_process_usercmds = unsafe extern "C" fn(this: *const CPlayer, cmds: *const CUserCmd, numcmds: u32, unk: usize, totalcmds: u32, paused: c_char) where offset(0x5a81c0);
        create_null_user_cmd = unsafe extern "C" fn(*mut CUserCmd) -> *mut CUserCmd where offset(0x25f790);
        player_run_command = unsafe extern "C" fn(*mut CPlayer, *mut CUserCmd,*const CMoveHelperServer) -> () where offset(0x5a7d80);
        fun_1805dd440 = unsafe extern "C" fn(*mut CPlayer) -> () where offset(0x5dd440);
        set_base_time = unsafe extern "C" fn(*mut CPlayer, f32) where offset(0x5b3790);
        set_last_cmd = unsafe extern "C" fn(*mut CUserCmd, *mut CUserCmd) -> () where offset(0x25f860);
        get_move_helper = unsafe extern "C" fn() -> *const CMoveHelperServer where offset(0x1b56f0);
        get_player_by_index = PlayerByIndex where offset(0x26AA10);
        util_get_command_client = unsafe extern "C" fn() -> *mut CPlayer where offset(0x15bf40);
        command_client_index = *const i32 where offset(0xbfbd84);
        interface_regs = *const InterfaceReg where offset(0x01752038);
        get_eye_pos = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x0043b8d0);
        get_center_pos = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x00407d30); // found these by pocking around in a vtable :)
        get_angles_01 = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x00442ce0);
        get_angles = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x0043c030);
        get_origin_varient = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x00443e80);
        get_origin = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x004198d0);
        eye_angles = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *const Vector3 where offset(0x4455f0); // this acceses the vtable
        view_angles = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *const Vector3 where offset(0x5d3960); // this acceses the vtable
        calc_absolute_velocity = unsafe extern "C" fn(*const CPlayer, *const *const CPlayer, usize, usize) -> () where offset(0x40a1e0);
        get_smoothed_velocity = unsafe extern "C" fn(*const CPlayer, *mut Vector3) -> *mut Vector3 where offset(0x58dfc0);
        calc_origin = unsafe extern "C" fn(*const CPlayer, *const *const CPlayer, usize, usize) -> () where offset(0x409ae0);

        set_origin = unsafe extern "C" fn(*const CPlayer, *const Vector3) where offset(0x433a80);
        set_origin_hack_do_not_use = unsafe extern "C" fn(*const CPlayer, *const Vector3) where offset(0x42dd30);
        check_position = unsafe extern "C" fn(*const Vector3) -> *const u8 where offset(0x438a90);
        perform_collision_check = unsafe extern "C" fn(*const CPlayer, u32) where offset(0x441480);
        another_perform_collision_check = unsafe extern "C" fn(*const CPlayer, *const CPlayer) where offset(0x443bd0);

        is_on_ground = unsafe extern "C" fn(*const CBaseEntity) -> usize where offset(0x441c60);
        is_alive = unsafe extern "C" fn(*const CBaseEntity) -> usize where offset(0x4461e0);
        is_titan = unsafe extern "C" fn(*const CBaseEntity) -> bool where offset(0x406a70);
        set_health = unsafe extern "C" fn(*mut CPlayer, i32, usize, usize) -> () where offset(0x42d7f0);
        create_script_instance = unsafe extern "C" fn(*mut CBaseEntity) -> *const SQObject where offset(0x43f2f0);
        get_player_net_int = unsafe extern "C" fn(*const CPlayer, *const c_char) -> i32 where offset(0x5ddc30);
        get_net_var_from_ent = unsafe extern "C" fn(*const CBaseEntity, *const c_char, i32, *mut i32) -> i32 where offset(0x1fa9c0);
        get_entity_name = unsafe extern "C" fn(*const CPlayer) -> *const c_char where offset(0x4179b0);
        ent_list = *const CEntInfo where offset(0x112d770);
        find_next_entity_by_class_name = unsafe extern "C" fn(*const c_void, *const CBaseEntity, *const c_char) -> *mut CBaseEntity where offset(0x44fdc0);
        some_magic_function_for_class_name = unsafe extern "C" fn(*mut *const c_char, *const c_char) -> *const *const c_char where offset(0x199e70);
        get_ent_by_script_name = unsafe extern "C" fn(*const c_void, *const c_char, *mut i32) -> *mut CBaseEntity where offset(0x455030);
        get_parent = unsafe extern "C" fn(*const CBaseEntity) -> *mut CBaseEntity where offset(0x445d50);

        get_offhand_weapon = unsafe extern "C" fn(*const CPlayer,u32 ) -> bool where offset(0xe1ec0); // not done
        set_weapon_by_slot = unsafe extern "C" fn(*const c_void, *const c_char) where offset(0xe4ba0);
        replace_weapon = unsafe extern "C" fn(*const CPlayer, *const c_char, *const c_void, *const c_void) where offset(0xdbae0);
        get_active_weapon = unsafe extern "C" fn(*const CPlayer) -> *const CBaseEntity where offset(0xea4c0);
        weapon_names_string_table = *const *const CNetworkStringTable where offset(0xbfbcf0);
        get_weapon_type = unsafe extern "C" fn(*const CBaseEntity) -> u32 where offset(0xf0cd0);
        get_weapon_charge_fraction = unsafe extern "C" fn(*const CBaseEntity) -> f32 where offset(0x68ea20);

        util_trace_line = unsafe extern "C" fn(*const Vector3, *const Vector3, c_char, c_char, i32, i32, i32, *mut CGameTrace )  where offset(0x2725c0);
        ctraceengine = *const *const *const fn() where offset(0xbfbdc8);
        simple_filter_vtable = *const fn() where offset(0x8ebbf8);
        create_trace_hull = unsafe extern "C" fn(this: *mut Ray, start: *const Vector3, end: *const Vector3, min: *const Vector3, max: *const Vector3) where offset(0x0ba0d0);

        draw_debug_line = unsafe extern "C" fn(point1: *const Vector3, point2: *const Vector3, r: i32, g: i32, b: i32, throught_walls: bool, time: f32) where offset(0x001ccf40);

        ent_fire = unsafe extern "C" fn(entity_instance: *mut CBaseEntity,input_namee: *const c_char, args: *const c_void, delay: f32, other_entity: *mut CBaseEntity, unk_or_null: *const c_void, unk:c_char ) where offset(0x29ea70);

        register_con_command = unsafe extern "C" fn(concommand: *mut ConCommand,name: *const c_char, callback: FnCommandCallback_t, help_string: *const c_char,flags: i32, completion: unsafe extern "C-unwind" fn(arg1: *const ::std::os::raw::c_char, arg2: *mut [::std::os::raw::c_char; 128usize]) -> ::std::os::raw::c_int) -> *mut ConCommand where offset(0x723fa0);

        get_pet_titan = unsafe extern "C" fn(*const CPlayer) -> *const CBaseEntity where offset(0x5dd940);

        sq_threadwakeup = unsafe extern "C" fn(sqvm: *const HSquirrelVM, i32, *const c_void, *const HSquirrelVM) -> SQRESULT where offset(0x8780);
        sq_suspendthread = unsafe extern "C" fn(sqvm: *const HSquirrelVM, *const *mut c_void, usize, *const HSquirrelVM) -> SQRESULT where offset(0x434f0);

        some_global_for_threads = *mut c_void where offset(0x23683c8);
        fun_180042560 = unsafe extern "C" fn(*const *mut (), f32) -> *const HSquirrelVM where offset(0x42560);
        somehow_suspend_thread = unsafe extern "C" fn(*const HSquirrelVM) where offset(0x44660);

        decoy_set_state = unsafe extern "C" fn(*mut CPlayerDecoy, i32) where offset(0x1c7af0);
        decoy_set_orientation = unsafe extern "C" fn(*const CPlayerDecoy, *mut [Vector3;8], *const Vector3, *const Vector3) where offset(0x4320e0);
        decoy_set_modifiers = unsafe extern "C" fn(*const CPlayerDecoy, i32) where offset(0x1c34d0);
        direction_to_angles = unsafe extern "C" fn(*const Vector3, *mut Vector3, *mut Vector3) where offset(0x6f8f20);
        is_in_some_busy_interaction = unsafe extern "C" fn(*const CPlayer) -> bool where offset(0x5d4d40);
    }
}
// very intersting call at server.dll + 0x151782
// call that possibly sets 1 max player for sp? : 0x15191a server.dll

offset_functions! {
    CLIENT_FUNCTIONS + ClientFunctions for WhichDll::Client => {
        base = *const c_void where offset(0x0);
        get_c_player_by_index = unsafe extern "C" fn(i32) -> *mut C_Player where offset(0x348650);
        get_local_c_player = unsafe extern "C" fn() -> *mut C_Player where offset(0x14ef40);
        c_player_get_name = unsafe extern "C" fn(*const C_Player) -> *const c_char where offset(0x14f320);
    }
}

offset_functions! {
    MATSYS_FUNCTIONS + MatSysFunctions for WhichDll::Other("materialsystem_dx11.dll") => {
        some_ctexture_function = SomeCtextureFunction where offset(0x00079e80);
    }
}
