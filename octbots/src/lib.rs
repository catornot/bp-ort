#![feature(let_chains, mpmc_channel)]

use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
use rrplug::{
    bindings::plugin_abi::PluginColor, exports::windows::Win32::Foundation::HMODULE,
    mid::northstar::NORTHSTAR_DATA, prelude::*,
};
use shared::{
    bindings::{
        ClientFunctions, EngineFunctions, HostState, MatSysFunctions, ServerFunctions,
        CLIENT_FUNCTIONS, ENGINE_FUNCTIONS, MATSYS_FUNCTIONS, SERVER_FUNCTIONS,
    },
    interfaces::{IVDebugOverlay, IVEngineServer},
    plugin_interfaces::{rust_version_hash, ExternalSimulations},
};
use std::{ffi::CStr, sync::Arc};
// use tracing_chrome::FlushGuard;
// use tracing_subscriber::layer::SubscriberExt;

use crate::async_pathfinding::JobMarket;

mod async_pathfinding;
mod behavior;
mod loader;
mod pathfinding;

const PLUGIN_DLL_NAME: *const i8 = c"octbots.dll".as_ptr();

pub static ENGINE_INTERFACES: OnceCell<SourceInterfaces> = OnceCell::new();

pub struct SourceInterfaces {
    pub debug_overlay: &'static IVDebugOverlay, // since it's a ptr to class which has a ptr to vtable
    pub engine_server: &'static IVEngineServer,
}

unsafe impl Sync for SourceInterfaces {}
unsafe impl Send for SourceInterfaces {}

pub struct OctBots {
    #[allow(clippy::type_complexity)]
    navmesh: Arc<RwLock<loader::Navmesh>>,
    current_map: Mutex<String>,
    simulations: OnceCell<&'static ExternalSimulations>,
    job_market: JobMarket,
    // frame: Mutex<UnsafeHandle<EnteredSpan>>, // I really need this
    // trace_guard: Mutex<Option<FlushGuard>>,
}

impl Plugin for OctBots {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new_with_color(
        c"octbots",
        c" OCTBOTS ",
        c"OCTBOTS",
        PluginContext::all(),
        PluginColor {
            red: 255,
            green: 127,
            blue: 127,
        },
    );

    fn new(_reloaded: bool) -> Self {
        let navmesh = Arc::new(RwLock::new(loader::Navmesh::default()));

        // let file = PathBuf::from(format!(
        //     "trace/trace_{}.json",
        //     SystemTime::now()
        //         .duration_since(SystemTime::UNIX_EPOCH)
        //         .unwrap()
        //         .as_secs_f64()
        //         .trunc() as i64
        // ));

        // fs::create_dir_all(file.parent().unwrap()).unwrap();
        // _ = fs::File::create_new(&file);

        // log::info!("trace goes into {file:?}");

        // let (chrome_layer, guard) = tracing_chrome::ChromeLayerBuilder::new().file(file).build();
        // tracing::subscriber::set_global_default(tracing_subscriber::registry().with(chrome_layer))
        //     .expect("setup tracy layer");

        Self {
            current_map: Mutex::new("".to_string()),
            job_market: JobMarket::new(Arc::clone(&navmesh)),
            navmesh,
            simulations: OnceCell::new(),
            // frame: Mutex::new(unsafe { UnsafeHandle::new(Span::none().entered()) }),
            // trace_guard: Mutex::new(Some(guard)),
        }
    }

    fn plugins_loaded(&self, _engine_token: EngineToken) {
        if let Some(interface) =
            unsafe { ExternalSimulations::from_dll_name("bp_ort.dll", "ExternalSimulation001") }
                .iter()
                .find(|interface| unsafe { interface.rust_version_hash() == rust_version_hash() })
        {
            unsafe {
                if !interface.set_bot_init(PLUGIN_DLL_NAME, behavior::init_bot) {
                    log::error!("failed to register init_bot function");
                }

                if !interface.register_simulation(
                    PLUGIN_DLL_NAME,
                    100,
                    behavior::wallpathfining_bots,
                ) {
                    log::error!("failed to register a simulation function");
                }
            };

            log::info!(
                "loaded interfaces {} {}",
                unsafe { interface.rust_version_hash() },
                rust_version_hash()
            );
            _ = self.simulations.set(interface);
        } else {
            log::error!("failed to load interfaces from bp_ort; unloading!");
            log::error!(
                "possibly because the plugin doesn't exist or was compiled on a different version of rustc"
            );
            let northstar_data = NORTHSTAR_DATA
                .get()
                .expect("northstar interface should exist");
            unsafe { northstar_data.sys().unload(northstar_data.handle()) };
        }
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, _: EngineToken) {
        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        let Some(_) = engine else { return };

        _ = unsafe {
            ENGINE_INTERFACES.set(SourceInterfaces {
                debug_overlay: IVDebugOverlay::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VDebugOverlay004",
                )
                .unwrap(),
                engine_server: IVEngineServer::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VEngineServer022",
                )
                .unwrap(),
            })
        };
    }

    fn on_sqvm_destroyed(&self, sqvm_handle: &CSquirrelVMHandle, _engine_token: EngineToken) {
        if sqvm_handle.get_context() != ScriptContext::SERVER {
            return;
        }

        // self.trace_guard.lock().take();
        behavior::drop_behaviors();
    }

    fn on_reload_request(&self) -> reloading::ReloadResponse {
        if let Some(mut navmesh) = self.navmesh.try_write() {
            navmesh.drop_navmesh();
        }

        unsafe { self.simulations.wait().drop_simulation(PLUGIN_DLL_NAME) };
        self.job_market.stop();
        // self.trace_guard.lock().flush();

        // has to be reloadable
        unsafe { reloading::ReloadResponse::allow_reload() }
    }

    fn runframe(&self, _engine_token: EngineToken) {
        // let mut frame = self.frame.lock();
        // *frame = unsafe { UnsafeHandle::new(span!(Level::TRACE, "frame").entered()) };

        if let Some(state) = unsafe {
            ENGINE_FUNCTIONS
                .get()
                .and_then(|funcs| funcs.host_state.as_ref())
        } {
            let current_name =
                unsafe { CStr::from_ptr(state.level_name.as_ptr()).to_string_lossy() };
            let mut load_nav = self.current_map.lock();

            if *load_nav != current_name && state.current_state == HostState::Run {
                self.navmesh.write().load_navmesh(current_name.as_ref());

                *load_nav = current_name.to_string();
            } else if let Some(mut navmesh) = self.navmesh.try_write() {
                match &navmesh.navmesh {
                    loader::NavmeshStatus::Unloaded => {}
                    loader::NavmeshStatus::Loading => {
                        navmesh.try_loaded();
                    }
                    loader::NavmeshStatus::Loaded(_) => {}
                }
            }
        }
    }
}

entry!(OctBots);
