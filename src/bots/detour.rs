use retour::static_detour;
use rrplug::{
    bindings::class_types::{client::CClient, cplayer::CPlayer},
    mid::utils::str_from_char_ptr,
};
use std::{
    ffi::{c_char, c_uchar, c_void},
    mem,
};

use super::{
    cmds_exec::{replace_cmd, run_bots_cmds},
    set_on_join::set_stuff_on_join,
};
use crate::bindings::{CMoveHelperServer, CUserCmd, SERVER_FUNCTIONS};

static_detour! {
    static Physics_RunThinkFunctions: unsafe extern "C" fn(bool);
    // static CClient__Connect: unsafe extern "C" fn(CClientPtr, *const c_char, *const c_void, c_char, *const c_void, [c_char;256] this is a *mut c_char, *const c_void ) -> bool;
    static SomeFuncInConnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void);
    static CreateNullUserCmd: unsafe extern "C" fn(*mut CUserCmd) -> *mut CUserCmd;
    static SomeFuncInDisconnectProcedure: unsafe extern "C" fn(*mut CClient, *const c_void,c_uchar);
    static CClient__Disconnect: unsafe extern "C" fn(*mut CClient, c_uchar, *const c_void, *const c_void);
    static FUN_18069e7a0: unsafe extern "C" fn(*mut c_void, *mut CPlayer, *const c_void);
    static CMoveHelperServer__PlayerFallingDamage: unsafe extern "C" fn(*mut CMoveHelperServer, *mut c_void, *const c_void, *const c_void);
    static GetPlayerNetInt: unsafe extern "C" fn(*mut CPlayer, *const c_char)-> i32 ;
    static GetNetVarFromEnt: unsafe extern "C" fn(*mut CPlayer, *const c_char, i32, *const i32) -> usize ;
}

fn physics_run_think_functions_hook(paused: bool) {
    run_bots_cmds(paused);

    unsafe { Physics_RunThinkFunctions.call(paused) }
}

fn create_null_cmd_hook(cmd: *mut CUserCmd) -> *mut CUserCmd {
    replace_cmd()
        .map(|new_cmd| {
            unsafe { *cmd = *new_cmd };
            cmd
        })
        .unwrap_or_else(|| unsafe { CreateNullUserCmd.call(cmd) })
}

fn get_player_net_int_hook(player: *mut CPlayer, var: *const c_char) -> i32 {
    unsafe {
        log::info!(
            "ent: {:?}",
            str_from_char_ptr((SERVER_FUNCTIONS.wait().get_entity_name)(player))
        );
        log::info!("var: {:?}", str_from_char_ptr(var));
        let value = GetPlayerNetInt.call(player, var);

        log::info!("value: {value}");

        value
    }
}

fn get_net_var_from_ent_hook(
    player: *mut CPlayer,
    var: *const c_char,
    index: i32,
    unk1: *const i32,
) -> usize {
    unsafe {
        log::info!(
            "ent: {:?}",
            str_from_char_ptr((SERVER_FUNCTIONS.wait().get_entity_name)(player))
        );
        log::info!("var: {:?}", str_from_char_ptr(var));
        log::info!("index: {:?}", index);
        let value = GetNetVarFromEnt.call(player, var, index, unk1);

        log::info!("value: {value}");

        value
    }
}

// maybe also check in FUN_1805d52f0 for null player

// for some reason the next 2 functions can randmoly get null pointers when bots are in titans
fn fun_18069e7a0_hook(unk1: *mut c_void, player: *mut CPlayer, unk2: *const c_void) {
    unsafe {
        let player = player
            .as_mut()
            .expect("like fr? why is the player null here FUN_69e7a0");

        if player.current_command.is_null() {
            return;
        }

        FUN_18069e7a0.call(unk1, player, unk2)
    }
}

fn player_falling_damage_hook(
    this: *mut CMoveHelperServer,
    unk2: *mut c_void,
    unk3: *const c_void,
    unk4: *const c_void,
) {
    unsafe {
        let this = this
            .as_mut()
            .expect("like fr? why is the null here FUN_69e7a0");

        if this.host.is_null() {
            return;
        }

        CMoveHelperServer__PlayerFallingDamage.call(this, unk2, unk3, unk4)
    }
}

pub fn hook_server(addr: *const c_void) {
    log::info!("hooking bot server functions");

    // netvar_hook_server(addr);

    unsafe {
        GetPlayerNetInt
            .initialize(
                mem::transmute(addr.offset(0x5ddc30)),
                get_player_net_int_hook,
            )
            .expect("failed to hook GetPlayerNetInt");
        // .enable()
        // .expect("failure to enable the GetPlayerNetInt");

        log::info!("hooked GetPlayerNetInt");

        GetNetVarFromEnt
            .initialize(
                mem::transmute(addr.offset(0x1fa9c0)),
                get_net_var_from_ent_hook,
            )
            .expect("failed to hook GetNetVarFromEnt");
        // .enable()
        // .expect("failure to enable the GetNetVarFromEnt");

        log::info!("hooked GetNetVarFromEnt");

        FUN_18069e7a0
            .initialize(mem::transmute(addr.offset(0x69e7a0)), fun_18069e7a0_hook)
            .expect("failed to hook FUN_18069e7a0")
            .enable()
            .expect("failure to enable the FUN_18069e7a0");

        log::info!("hooked FUN_18069e7a0");

        CMoveHelperServer__PlayerFallingDamage
            .initialize(
                mem::transmute(addr.offset(0x1b5720)),
                player_falling_damage_hook,
            )
            .expect("failed to hook CMoveHelperServer__PlayerFallingDamage")
            .enable()
            .expect("failure to enable the CMoveHelperServer__PlayerFallingDamage");

        log::info!("hooked CMoveHelperServer__PlayerFallingDamage");

        Physics_RunThinkFunctions
            .initialize(
                mem::transmute(addr.offset(0x483A50)),
                physics_run_think_functions_hook,
            )
            .expect("failed to hook Physics_RunThinkFunctions")
            .enable()
            .expect("failure to enable the Physics_RunThinkFunctions hook");

        log::info!("hooked Physics_RunThinkFunctions");

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

pub fn hook_engine(addr: *const c_void) {
    log::info!("hooking bot engine functions");

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
    }
}

// cool init funtion may be usful to allow people to join singleplayer
// 0x1145bd
// and this to set singleplayer player cap?
// 0x156c86
