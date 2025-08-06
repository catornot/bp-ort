use crate::{bindings::CUserCmd, cmds_helper::CUserCmdHelper};
use rrplug::{bindings::class_types::cplayer::CPlayer, create_external_interface};
use std::ffi::c_char;

pub type SimulationFunc = extern "C" fn(&CUserCmdHelper, &mut CPlayer) -> CUserCmd;

create_external_interface! {
    pub ExternalSimulations + ExternalSimulations001 => {
        pub fn rust_version_hash() -> u64;
        pub fn register_simulation(dll_name: *const c_char, simtype: usize, func: SimulationFunc) -> bool;
        pub fn unregister_simulation(dll_name: *const c_char, simtype: usize) -> bool;
        pub fn drop_simulation(dll_name: *const c_char) -> bool;
    }
}

pub fn rust_version_hash() -> u64 {
    include!(concat!(env!("OUT_DIR"), "/rustc"))
}

unsafe impl Sync for ExternalSimulations {}
unsafe impl Send for ExternalSimulations {}
