#![feature(unboxed_closures, layout_for_ptr)]

use bots::Bots;
use rrplug::prelude::*;
use rrplug::wrappers::northstar::{EngineLoadType, PluginData};
use std::mem;
use std::sync::Mutex;
use tf2dlls::SourceEngineData;

mod bots;
mod hooks;
mod native_types;
mod structs;
mod tf2dlls;

// todo put these into folders

mod screen_detour;

#[derive(Debug)]
pub struct HooksPlugin {
    pub source_engine_data: Mutex<SourceEngineData>,
    pub bots: Bots,
}

impl Plugin for HooksPlugin {
    fn new() -> Self {
        Self {
            #[allow(invalid_value)]
            source_engine_data: Mutex::new(unsafe { mem::MaybeUninit::zeroed().assume_init() }),
            bots: Bots::new(),
        }
    }

    fn initialize(&mut self, _plugin_data: &PluginData) {}

    fn main(&self) {}

    fn on_engine_load(&self, engine: &EngineLoadType) {
        self.bots.on_engine_load(engine);

        match engine {
            EngineLoadType::Engine(_) => {},
            EngineLoadType::EngineFailed => return,
            EngineLoadType::Server => {
                std::thread::spawn(|| {
                    wait(10000);

                    let plugin = PLUGIN.wait();
                    plugin
                        .source_engine_data
                        .lock()
                        .expect("how")
                        .load_server(vec![&plugin.bots])
                });
                return;
            }
            EngineLoadType::Client => {
                self.source_engine_data
                    .lock()
                    .expect("how")
                    .load_materialsystem();

                std::thread::spawn(|| {
                    wait(10000);

                    let plugin = PLUGIN.wait();
                    plugin
                        .source_engine_data
                        .lock()
                        .expect("how")
                        .load_client(vec![&plugin.bots])
                });
                return;
            }
        };

        self.source_engine_data
            .lock()
            .expect("how")
            .load_engine(vec![&self.bots]);
    }
}

entry!(HooksPlugin);
