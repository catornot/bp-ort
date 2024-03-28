use rrplug::prelude::*;

use self::concommands::register_concommands;

mod concommands;

#[derive(Debug)]
pub struct Disguise;

impl Plugin for Disguise {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"disguise",
        c"disguise",
        c"disguise",
        PluginContext::DEDICATED,
    );
    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, _dll_ptr: &DLLPointer, token: EngineToken) {
        let Some(engine) = engine else { return };

        register_concommands(engine, token)
    }
}
