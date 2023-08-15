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
}
