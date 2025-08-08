#![feature(let_chains)]

use bincode::Decode;
use loader::map_to_i32;
use oktree::prelude::*;
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

#[derive(Decode)]
pub struct NavmeshBin {
    min: [i32; 3],
    max: [i32; 3],
    cell_size: f32,
    filled_pos: Vec<[i32; 3]>,
}

pub struct OctBots {
    #[allow(clippy::type_complexity)]
    navmesh: Arc<RwLock<loader::Navmesh>>,
    current_map: Mutex<String>,
    simulations: OnceCell<&'static ExternalSimulations>,
}

impl Plugin for OctBots {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new_with_color(
        c"octbots",
        c" OCTBOTS ",
        c"OCTBOTS",
        PluginContext::all(),
        PluginColor {
            red: 139,
            green: 0,
            blue: 139,
        },
    );

    fn new(_reloaded: bool) -> Self {
        Self {
            current_map: Mutex::new("".to_string()),
            navmesh: Arc::new(RwLock::new(loader::Navmesh::default())),
            simulations: OnceCell::new(),
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

        behavior::drop_behaviors();
    }

    fn on_reload_request(&self) -> reloading::ReloadResponse {
        if let Some(mut navmesh) = self.navmesh.try_write() {
            navmesh.drop_navmesh();
        }

        unsafe { self.simulations.wait().drop_simulation(PLUGIN_DLL_NAME) };

        // has to be reloadable
        unsafe { reloading::ReloadResponse::allow_reload() }
    }

    #[allow(unused_variables, unreachable_code)]
    fn runframe(&self, _engine_token: EngineToken) {
        if let Some((state, origin)) = unsafe {
            ENGINE_FUNCTIONS
                .get()
                .and_then(|funcs| funcs.host_state.as_ref())
                .and_then(|state| Some((state, SERVER_FUNCTIONS.get()?)))
                .and_then(|(state, server_funcs)| {
                    let mut v = Vector3::ZERO;
                    Some((
                        state,
                        *(server_funcs.get_player_by_index)(1)
                            .as_ref()?
                            .get_origin(&mut v),
                    ))
                })
        } {
            let current_name =
                unsafe { CStr::from_ptr(state.level_name.as_ptr()).to_string_lossy() };
            let mut load_nav = self.current_map.lock();

            if *load_nav != current_name && state.current_state == HostState::Run {
                self.navmesh.write().load_navmesh(current_name.as_ref());

                *load_nav = current_name.to_string();
            } else {
                let Some(debug) = ENGINE_INTERFACES.get().map(|engine| engine.debug_overlay) else {
                    return;
                };

                let mut navmesh = self.navmesh.write();

                let octree = match &navmesh.navmesh {
                    loader::NavmeshStatus::Unloaded => return,
                    loader::NavmeshStatus::Loading => {
                        navmesh.try_loaded();
                        return;
                    }
                    loader::NavmeshStatus::Loaded(octree) => octree,
                };

                let cell_size = navmesh.cell_size;

                pub fn distance3(pos: Vector3, target: Vector3) -> f32 {
                    ((pos.x - target.x).powi(2)
                        + (pos.y - target.y).powi(2)
                        + (pos.z - target.z).powi(2))
                    .sqrt()
                }

                return;

                for origin in octree
                    .iter_elements()
                    .map(|(_, point)| tuvec_to_vector3(cell_size, *point))
                    .filter(|pos| distance3(*pos, origin) < 500.)
                {
                    let half_cube = cell_size / 2.;
                    // unsafe {
                    // debug.AddLineOverlay(
                    //     &(Vector3::new(-half_cube, -half_cube, -half_cube) + origin),
                    //     &(Vector3::new(half_cube, half_cube, half_cube) + origin),
                    //     255,
                    //     20,
                    //     20,
                    //     false,
                    //     0.1,
                    // );
                    // debug.AddBoxOverlay(
                    //     &origin,
                    //     &Vector3::new(-half_cube, -half_cube, -half_cube),
                    //     &Vector3::new(half_cube, half_cube, half_cube),
                    //     &Vector3::new(0., 0., 0.),
                    //     0,
                    //     0,
                    //     200,
                    //     10,
                    //     false,
                    //     0.1,
                    // );
                    // };
                }

                for (min, max) in octree
                    .iter_nodes()
                    .map(|node| {
                        (
                            tuvec_to_vector3(cell_size, TUVec3u32(node.aabb.min)),
                            tuvec_to_vector3(cell_size, TUVec3u32(node.aabb.max)),
                        )
                    })
                    .filter(|pos| {
                        distance3(pos.0, origin) < 100. || distance3(pos.1, origin) < 100.
                    })
                {
                    // unsafe {
                    //     debug.AddLineOverlay(
                    //         &(Vector3::new(min.x, 0., 0.)),
                    //         &(Vector3::new(max.x, 0., 0.)),
                    //         255,
                    //         200,
                    //         20,
                    //         false,
                    //         0.1,
                    //     );
                    //     debug.AddLineOverlay(
                    //         &(Vector3::new(0., min.y, 0.)),
                    //         &(Vector3::new(0., max.y, 0.)),
                    //         255,
                    //         200,
                    //         20,
                    //         false,
                    //         0.1,
                    //     );
                    //     debug.AddLineOverlay(
                    //         &(Vector3::new(0., 0., min.z)),
                    //         &(Vector3::new(0., 0., max.z)),
                    //         255,
                    //         200,
                    //         20,
                    //         false,
                    //         0.1,
                    //     );
                    //     debug.AddLineOverlay(
                    //         &(Vector3::new(min.x, min.y, min.z)),
                    //         &(Vector3::new(max.x, max.y, max.z)),
                    //         255,
                    //         200,
                    //         20,
                    //         false,
                    //         0.1,
                    //     );
                    // };
                }
            }
        }
    }
}

fn tuvec_to_vector3(cell_size: f32, point: TUVec3u32) -> Vector3 {
    Vector3::new(
        map_to_i32(point.0.x) as f32,
        map_to_i32(point.0.y) as f32,
        map_to_i32(point.0.z) as f32,
    ) * Vector3::new(cell_size, cell_size, cell_size)
}

entry!(OctBots);
