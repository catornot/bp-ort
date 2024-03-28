#![feature(c_variadic, iter_array_chunks, iter_collect_into)]

use rrplug::prelude::*;

use crate::{
    admin_abuse::AdminAbuse,
    bindings::{
        ClientFunctions, EngineFunctions, MatSysFunctions, ServerFunctions, CLIENT_FUNCTIONS,
        ENGINE_FUNCTIONS, MATSYS_FUNCTIONS, SERVER_FUNCTIONS,
    },
    bots::Bots,
    devtoys::DevToys,
    disguise::Disguise,
    interfaces::Interfaces,
    navmesh::{RecastDetour, RECAST_DETOUR},
    screen_detour::hook_materialsystem,
};

mod admin_abuse;
mod bindings;
mod bots;
mod devtoys;
mod disguise;
mod interfaces;
mod navmesh;
mod screen_detour;
mod utils;

#[derive(Debug)]
pub struct HooksPlugin {
    pub bots: Bots,
    pub disguise: Disguise,
    pub interfaces: Interfaces,
    pub admin_abuse: AdminAbuse,
    pub devtoys: DevToys,
    is_dedicated_server: bool,
}

impl Plugin for HooksPlugin {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"whoks", c"WHOKS0000", c"WHOKS", PluginContext::all());

    fn new(reloaded: bool) -> Self {
        if reloaded {
            panic!("bad things will happen if this is reloaded")
        }

        Self {
            bots: Bots::new(reloaded),
            disguise: Disguise::new(reloaded),
            interfaces: Interfaces::new(reloaded),
            admin_abuse: AdminAbuse::new(reloaded),
            devtoys: DevToys::new(reloaded),
            is_dedicated_server: std::env::args().any(|cmd| cmd.starts_with("-dedicated")),
        }
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        self.bots.on_dll_load(engine, dll_ptr, token);
        self.disguise.on_dll_load(engine, dll_ptr, token);
        self.interfaces.on_dll_load(engine, dll_ptr, token);
        self.admin_abuse.on_dll_load(engine, dll_ptr, token);
        self.devtoys.on_dll_load(engine, dll_ptr, token);

        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
            RecastDetour::try_init(dll_ptr, &RECAST_DETOUR);
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
                   // utils::patch(
                   //     base + 0x5aa01f,
                   //     &[0x90; 30], // 40 bytes
                   // ); // removes the world view write in run_null_command
            },
            _ => {}
        }
    }

    fn on_sqvm_created(&self, sqvm_handle: &CSquirrelVMHandle, token: EngineToken) {
        self.bots.on_sqvm_created(sqvm_handle, token);
        self.interfaces.on_sqvm_created(sqvm_handle, token);
        self.admin_abuse.on_sqvm_created(sqvm_handle, token);
    }

    fn on_sqvm_destroyed(&self, sqvm_handle: &CSquirrelVMHandle, token: EngineToken) {
        self.bots.on_sqvm_destroyed(sqvm_handle, token)
    }

    fn runframe(&self, token: EngineToken) {
        self.interfaces.runframe(token);
        self.devtoys.runframe(token);
    }
}

impl HooksPlugin {
    pub fn is_dedicated_server(&self) -> bool {
        self.is_dedicated_server
    }
}

entry!(HooksPlugin);
