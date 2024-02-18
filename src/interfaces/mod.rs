use libc::c_void;
use once_cell::sync::OnceCell;
use rrplug::prelude::*;

use crate::utils::create_source_interface;

use self::concommands::register_concommands;

mod concommands;
mod hooks;

pub static ENGINE_INTERFACES: OnceCell<EngineInterfaces> = OnceCell::new();

pub struct EngineInterfaces {
    pub debug_overlay: *mut *const [*const c_void; 31], // since it's a ptr to class which has a ptr to vtable
    pub engine_server: *mut *const [*const c_void; 211],
    pub engine_client: *mut *const [*const c_void; 200],
}

unsafe impl Sync for EngineInterfaces {}
unsafe impl Send for EngineInterfaces {}

#[derive(Debug)]
pub struct Interfaces;

impl Plugin for Interfaces {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        "Interfaces",
        "Interfaces",
        "Interfaces",
        PluginContext::all(),
    );

    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        hooks::hook(dll_ptr);

        let Some(engine) = engine else { return };

        register_concommands(engine, token);

        _ = unsafe {
            ENGINE_INTERFACES.set(EngineInterfaces {
                debug_overlay: create_source_interface::<*const [*const c_void; 31]>(
                    "engine.dll\0".as_ptr().cast(),
                    "VDebugOverlay004\0".as_ptr().cast(),
                )
                .unwrap(),
                engine_server: create_source_interface::<*const [*const c_void; 211]>(
                    "engine.dll\0".as_ptr().cast(),
                    "VEngineServer022\0".as_ptr().cast(),
                )
                .unwrap(),
                engine_client: create_source_interface::<*const [*const c_void; 200]>(
                    ("engine.dll\0").as_ptr().cast(),
                    ("VEngineClient013\0").as_ptr().cast(),
                )
                .unwrap(),
            })
        };
    }

    fn runframe(&self, token: EngineToken) {
        let Ok(convar) = ConVarStruct::find_convar_by_name("idcolor_ally", token) else {
            return;
        };

        let Ok(line) = convar.get_value_str() else {
            return;
        };

        let Some(color) = line.split(' ').next() else {
            return;
        };

        let Ok(value) = color.parse::<f32>() else {
            return;
        };

        convar.set_value_string(
            format!(
                "{:.*} 0.100 1.000 8",
                3,
                if value < 1. { value + 0.01 } else { 0. }
            ),
            token,
        )
    }
}
