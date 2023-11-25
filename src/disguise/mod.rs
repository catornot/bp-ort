use rrplug::prelude::*;

use self::concommands::register_concommands;

mod concommands;

#[derive(Debug)]
pub struct Disguise;

impl Plugin for Disguise {
    fn new(_plugin_data: &PluginData) -> Self {
        Self {}
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, _dll_ptr: &DLLPointer) {
        let Some(engine) = engine else { return };

        register_concommands(engine)
    }
}
