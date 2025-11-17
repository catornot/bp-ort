use rrplug::prelude::*;

mod sqapi;

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
        sqapi::digsuise_sqapi();
        Self {}
    }
}
