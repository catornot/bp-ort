use rrplug::prelude::*;

use self::concommands::register_concommands;

mod concommands;

#[derive(Debug)]
pub struct Interfaces;

impl Plugin for Interfaces {
    fn new(_plugin_data: &PluginData) -> Self {
        Self {
        }
    }

    fn main(&self) {}

    fn on_dll_load(&self, engine: &PluginLoadDLL, _dll_ptr: &DLLPointer) {
        let engine = match engine {
            PluginLoadDLL::Engine(engine) => engine,
            _ => return,
        };

        register_concommands(engine)
    }
}