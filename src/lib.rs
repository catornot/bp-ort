#![feature(result_option_inspect)]

use admin_abuse::AdminAbuse;
use rrplug::prelude::*;

mod admin_abuse;
mod bindings;
mod bots;
mod disguise;
mod interfaces;
mod utils;

use crate::{
    bindings::{
        ClientFunctions, EngineFunctions, MatSysFunctions, ServerFunctions, CLIENT_FUNCTIONS,
        ENGINE_FUNCTIONS, MATSYS_FUNCTIONS, SERVER_FUNCTIONS,
    },
    bots::Bots,
    disguise::Disguise,
    interfaces::Interfaces,
    screen_detour::hook_materialsystem,
};

mod screen_detour;

#[derive(Debug)]
pub struct HooksPlugin {
    pub bots: Bots,
    pub disguise: Disguise,
    pub interfaces: Interfaces,
    pub admin_abuse: AdminAbuse,
    is_dedicated_server: bool,
}

impl Plugin for HooksPlugin {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new("whoks\0", "WHOKS0000\0", "WHOKS\0", PluginContext::all());

    fn new(reloaded: bool) -> Self {
        Self {
            bots: Bots::new(reloaded),
            disguise: Disguise::new(reloaded),
            interfaces: Interfaces::new(reloaded),
            admin_abuse: AdminAbuse::new(reloaded),
            is_dedicated_server: std::env::args().any(|cmd| cmd.starts_with("-dedicated")),
        }
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        self.bots.on_dll_load(engine, dll_ptr, token);
        self.disguise.on_dll_load(engine, dll_ptr, token);
        self.interfaces.on_dll_load(engine, dll_ptr, token);
        self.admin_abuse.on_dll_load(engine, dll_ptr, token);

        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        match dll_ptr.which_dll() {
            WhichDll::Other(other) if *other == "materialsystem_dx11.dll" => {
                hook_materialsystem(dll_ptr.get_dll_ptr())
            }
            WhichDll::Server => unsafe {
                let base = SERVER_FUNCTIONS.wait().base as usize;
                // patch(
                //     base + 0x5a8241,
                //     &[
                //         0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                //         0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                //         0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                //     ],
                // ); // removes the Client \'%s\' dropped %i packets to us spam
                // patch(
                //     base + 0x5a825d,
                //     &[
                //         0x90, 0x90, 0x90, 0x90, 0x90,
                //     ],
                // ); // same thing but less nops
                utils::patch(
                    base + 0x15191a,
                    &[0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90],
                ); // removes 1 max player on sp
            },
            _ => {}
        }
    }
}

impl HooksPlugin {
    pub fn is_dedicated_server(&self) -> bool {
        self.is_dedicated_server
    }
}

entry!(HooksPlugin);
