use rrplug::prelude::*;

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
    utils::patch,
};

// todo put these into folders

mod screen_detour;

#[derive(Debug)]
pub struct HooksPlugin {
    pub bots: Bots,
    pub disguise: Disguise,
    pub interfaces: Interfaces,
}

impl Plugin for HooksPlugin {
    fn new(plugin_data: &PluginData) -> Self {
        Self {
            bots: Bots::new(plugin_data),
            disguise: Disguise::new(plugin_data),
            interfaces: Interfaces::new(plugin_data),
        }
    }

    fn main(&self) {}

    fn on_dll_load(&self, engine: &PluginLoadDLL, dll_ptr: &DLLPointer) {
        self.bots.on_dll_load(engine, dll_ptr);
        self.disguise.on_dll_load(engine, dll_ptr);
        self.interfaces.on_dll_load(engine, dll_ptr);

        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        match engine {
            PluginLoadDLL::Other(other) if other == "materialsystem_dx11.dll" => {
                hook_materialsystem(dll_ptr.get_dll_ptr())
            }
            PluginLoadDLL::Server => unsafe {
                let base = SERVER_FUNCTIONS.wait().base as usize;
                patch(
                    base + 0x5a8241,
                    &[
                        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                        0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                    ],
                ); // removes the Client \'%s\' dropped %i packets to us spam
            },
            _ => {}
        }
    }

    fn runframe(&self) {}
}

entry!(HooksPlugin);
