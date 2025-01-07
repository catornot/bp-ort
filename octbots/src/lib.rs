use rrplug::prelude::*;

pub struct OctBots;

impl Plugin for OctBots {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"octbots", c" OCTBOTS ", c"OCTBOTS", PluginContext::all());

    fn new(_reloaded: bool) -> Self {
        log::info!("octbots is loaded 1212");
        Self {}
    }

    fn plugins_loaded(&self, _engine_token: EngineToken) {}

    fn on_reload_request(&self) -> reloading::ReloadResponse {
        // has to be reloadable
        unsafe { reloading::ReloadResponse::allow_reload() }
    }
}

entry!(OctBots);
