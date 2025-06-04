use rrplug::bindings::cvar::convar::FCVAR_CLIENTDLL;
use rrplug::mid::engine::concommands::find_concommand;
use rrplug::mid::utils::try_cstring;
use rrplug::prelude::*;
use std::mem;
use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

use crate::bindings::{CreateInterfaceFn, SERVER_FUNCTIONS};
use crate::utils::from_c_string;

pub fn register_concommands(engine: &EngineData, token: EngineToken) {
    engine
        .register_concommand(
            "interfaces_load_some",
            interfaces_load_some,
            "",
            FCVAR_CLIENTDLL as i32,
            token,
        )
        .expect("couldn't register concommand");

    engine
        .register_concommand(
            "interfaces_server",
            interfaces_server,
            "",
            FCVAR_CLIENTDLL as i32,
            token,
        )
        .expect("couldn't register concommand");

    engine
        .register_concommand(
            "interfaces_player",
            interfaces_player,
            "",
            FCVAR_CLIENTDLL as i32,
            token,
        )
        .expect("couldn't register concommand");

    engine
        .register_concommand(
            "interfaces_test_player",
            interfaces_test_player,
            "",
            FCVAR_CLIENTDLL as i32,
            token,
        )
        .expect("couldn't register concommand");
    engine
        .register_concommand(
            "interfaces_show_ent_fire",
            interfaces_show_ent_fire,
            "",
            0,
            token,
        )
        .expect("couldn't register concommand");
}

#[rrplug::concommand]
pub fn interfaces_load_some(command: CCommandResult) -> Option<()> {
    let dll_name = try_cstring(command.get_arg(0)?).ok()?;

    let interface_name = command.get_args().get(1)?;
    let c_interface_name = try_cstring(interface_name).ok()?;

    unsafe {
        let dll = GetModuleHandleA(dll_name.as_ptr() as *const u8);

        if dll == 0 {
            return None;
        }

        let create_interface: Option<CreateInterfaceFn> =
            mem::transmute(GetProcAddress(dll, c"CreateInterface".as_ptr().cast()));

        let interface = (create_interface?)(c_interface_name.as_ptr(), std::ptr::null_mut());

        if !interface.is_null() {
            log::info!("found {interface_name} at addr {interface:?}");
        }
    }

    None
}

#[rrplug::concommand]
pub fn interfaces_server(_command: CCommandResult) -> Option<()> {
    let mut current = SERVER_FUNCTIONS.wait().interface_regs;
    while !current.is_null() {
        unsafe {
            let interface = &*current;

            if interface.name.is_null() {
                log::info!("valid interface with null name")
            } else {
                log::info!(
                    "interface {} at {:?}",
                    from_c_string::<String>(interface.name),
                    current
                );
            }

            current = interface.next;
        }
    }

    None
}

#[rrplug::concommand]
pub fn interfaces_player() -> Option<()> {
    unsafe {
        let server_functions = SERVER_FUNCTIONS.wait();
        let player = (server_functions.get_player_by_index)(1).as_mut()?;
        let base = server_functions.base as usize;

        log::info!("base : {base:X}");

        // let vtable = **player.vtable as *const _ as *const *const c_void;
        let vtable_array = player.vftable as *const _ as *const [usize; 214];

        for (i, ptr) in (*vtable_array).iter().enumerate() {
            if ptr - base == 0x5A9FD0 {
                log::info!("run_null_command function at {ptr:X}");
            }

            let ptr = ptr - base;

            log::info!("some vtable function at {ptr:X} {i}");
        }

        // // finding PlayerRunCommand
        // let mut last: *const *const c_void = std::ptr::null();
        // let mut current = vtable;
        // let mut index = 0;

        // while !(*current).is_null() {
        //     if (*current) as usize == run_null_command as usize {
        //         log::info!(
        //             "maybe PlayerRunCommand {:?}",
        //             (*last).offset(-(base as isize))
        //         );
        //         log::info!("RunNullCommand {:?}", (*current).offset(-(base as isize)));
        //         break;
        //     }

        //     last = current;
        //     current = vtable.add(index);
        //     index += 1;
        // }
    }
    None
}

#[rrplug::concommand]
pub fn interfaces_test_player() -> Option<()> {
    unsafe {
        let server_functions = SERVER_FUNCTIONS.wait();
        let player = (server_functions.get_player_by_index)(1).as_mut()?;

        let mut v = Vector3::from([0., 0., 0.]);
        let same_v = (server_functions.get_eye_pos)(player, &mut v).as_mut()?;
        log::info!("get_eye_pos = {same_v:?}");

        _ = (server_functions.get_center_pos)(player, &mut v);
        log::info!("get_center_pos = {v:?}");

        let same_v = (server_functions.get_angles_01)(player, &mut v).as_mut()?;
        log::info!("get_angles_01 = {same_v:?}");

        let same_v = (server_functions.get_angles)(player, &mut v).as_mut()?;
        log::info!("get_angles = {same_v:?}");

        let same_v = (server_functions.get_origin_varient)(player, &mut v).as_mut()?;
        log::info!("get_origin_varient = {same_v:?}");

        let same_v = (server_functions.get_origin)(player, &mut v).as_mut()?;
        log::info!("get_origin = {same_v:?}");

        let same_v = (server_functions.eye_angles)(player, &mut v).as_ref()?;
        log::info!("eye_angles = {same_v:?}");

        let ent_fire: *const () =
            *((server_functions.base as usize + 0xb63300) as *const *const ());

        log::info!(
            "ent_fire true addr: {}",
            ent_fire as usize - server_functions.base as usize
        )
    }
    None
}

#[rrplug::concommand]
pub fn interfaces_show_ent_fire() -> Option<()> {
    let c = find_concommand("ent_fire").ok()?;
    log::info!("{}", c.unk0);
    None
}
