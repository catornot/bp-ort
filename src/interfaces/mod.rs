use libc::c_void;
use once_cell::sync::OnceCell;
use rrplug::{prelude::*, to_c_string};

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
    fn new(_plugin_data: &PluginData) -> Self {
        Self {}
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer) {
        hooks::hook(dll_ptr);

        let Some(engine) = engine else { return };

        register_concommands(engine);

        _ = unsafe {
            ENGINE_INTERFACES.set(EngineInterfaces {
                debug_overlay: create_source_interface::<*const [*const c_void; 31]>(
                    to_c_string!(const "engine.dll\0").as_ptr(),
                    to_c_string!(const "VDebugOverlay004\0").as_ptr(),
                )
                .unwrap(),
                engine_server: create_source_interface::<*const [*const c_void; 211]>(
                    to_c_string!(const "engine.dll\0").as_ptr(),
                    to_c_string!(const "VEngineServer022\0").as_ptr(),
                )
                .unwrap(),
                engine_client: create_source_interface::<*const [*const c_void; 200]>(
                    to_c_string!(const "engine.dll\0").as_ptr(),
                    to_c_string!(const "VEngineClient013\0").as_ptr(),
                )
                .unwrap(),
            })
        };
    }

    fn runframe(&self) {
        let Some(convar) = ConVarStruct::find_convar_by_name("idcolor_ally") else {
            return;
        };

        let Ok(line) = convar.get_value_string() else {
            return;
        };

        let Some(color) = line.split(' ').next() else {
            return;
        };

        let Ok(value) = color.parse::<f32>() else {
            return;
        };

        convar.set_value_string(format!(
            "{:.*} 0.100 1.000 8",
            3,
            if value < 1. { value + 0.01 } else { 0. }
        ))
    }
}
