use rrplug::prelude::*;

use self::concommands::register_concommands;

mod concommands;
mod hooks;

#[derive(Debug)]
pub struct Interfaces;

impl Plugin for Interfaces {
    fn new(_plugin_data: &PluginData) -> Self {
        Self {}
    }

    fn main(&self) {}

    fn on_dll_load(&self, engine: &PluginLoadDLL, dll_ptr: &DLLPointer) {
        hooks::hook(dll_ptr);

        let engine = match engine {
            PluginLoadDLL::Engine(engine) => engine,
            _ => return,
        };

        register_concommands(engine)
    }

    fn runframe(&self) {
        let Some(convar) = ConVarStruct::find_convar_by_name("idcolor_ally") else {
            return;
        };

        let Ok(line) = convar.get_value_string()else {
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
