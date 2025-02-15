#![allow(clippy::too_many_arguments)]

use once_cell::sync::OnceCell;
use rrplug::{exports::windows::Win32::Foundation::HMODULE, prelude::*};
use shared::interfaces::*;

use self::concommands::register_concommands;

mod concommands;
mod hooks;

pub static ENGINE_INTERFACES: OnceCell<EngineInterfaces> = OnceCell::new();

pub struct EngineInterfaces {
    pub debug_overlay: &'static IVDebugOverlay, // since it's a ptr to class which has a ptr to vtable
    pub engine_server: &'static IVEngineServer,
}

unsafe impl Sync for EngineInterfaces {}
unsafe impl Send for EngineInterfaces {}

#[derive(Debug)]
pub struct Interfaces;

impl Plugin for Interfaces {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"Interfaces",
        c"Interfaces",
        c"Interfaces",
        PluginContext::all(),
    );

    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_sqvm_created(&self, _sqvm_handle: &CSquirrelVMHandle, _engine_token: EngineToken) {}

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        hooks::hook(dll_ptr);

        let Some(engine) = engine else { return };

        register_concommands(engine, token);

        _ = unsafe {
            ENGINE_INTERFACES.set(EngineInterfaces {
                debug_overlay: IVDebugOverlay::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VDebugOverlay004",
                )
                .unwrap(),
                engine_server: IVEngineServer::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VEngineServer022",
                )
                .unwrap(),
            })
        };
    }
}
