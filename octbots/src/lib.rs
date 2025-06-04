use bincode::Decode;
use loader::Navmesh;
use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
use rrplug::{exports::windows::Win32::Foundation::HMODULE, prelude::*};
use shared::{
    bindings::{
        ClientFunctions, EngineFunctions, HostState, MatSysFunctions, ServerFunctions,
        CLIENT_FUNCTIONS, ENGINE_FUNCTIONS, MATSYS_FUNCTIONS, SERVER_FUNCTIONS,
    },
    interfaces::{IVDebugOverlay, IVEngineServer},
};
use std::ffi::CStr;

mod loader;

const NAV_CUBE_SIZE: f32 = 50.;
const HALF_CUBE: f32 = NAV_CUBE_SIZE / 2.;

pub static ENGINE_INTERFACES: OnceCell<EngineInterfaces> = OnceCell::new();

pub struct EngineInterfaces {
    pub debug_overlay: &'static IVDebugOverlay, // since it's a ptr to class which has a ptr to vtable
    pub engine_server: &'static IVEngineServer,
}

unsafe impl Sync for EngineInterfaces {}
unsafe impl Send for EngineInterfaces {}

#[derive(Decode)]
pub struct NavmeshBin {
    min: [i32; 3],
    max: [i32; 3],
    cell_size: f32,
    filled_pos: Vec<[i32; 3]>,
}

pub struct OctBots {
    #[allow(clippy::type_complexity)]
    nav_grid: RwLock<Vec<[i32; 3]>>,
    cell_size: RwLock<f32>,
    navmesh: RwLock<loader::Navmesh>,
    current_map: Mutex<String>,
}

impl Plugin for OctBots {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"octbots", c" OCTBOTS ", c"OCTBOTS", PluginContext::all());

    fn new(_reloaded: bool) -> Self {
        Self {
            nav_grid: RwLock::new(Vec::new()),
            current_map: Mutex::new("".to_string()),
            cell_size: RwLock::new(25.),
            navmesh: RwLock::new(loader::Navmesh::default()),
        }
    }

    fn plugins_loaded(&self, _engine_token: EngineToken) {}

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, _: EngineToken) {
        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        let Some(_) = engine else { return };

        _ = unsafe {
            ENGINE_INTERFACES.set(EngineInterfaces {
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

    fn on_reload_request(&self) -> reloading::ReloadResponse {
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
                if let Some(navmesh) = std::fs::File::open(format!("output/{current_name}.navmesh"))
                    .ok()
                    .and_then(|mut reader| {
                        bincode::decode_from_std_read::<NavmeshBin, _, _>(
                            &mut reader,
                            bincode::config::standard(),
                        )
                        .ok()
                    })
                {
                    self.nav_grid.write().clear();
                    self.nav_grid.write().extend(navmesh.filled_pos);
                    *self.cell_size.write() = navmesh.cell_size;

                    log::info!("loaded {current_name}");
                } else {
                    self.nav_grid.write().clear();
                    log::error!("couldn't load {current_name}");
                }
                *load_nav = current_name.to_string();
            } else {
                let Some(debug) = ENGINE_INTERFACES.get().map(|engine| engine.debug_overlay) else {
                    return;
                };
                let cell_size = *self.cell_size.read();

                pub fn distance3(pos: Vector3, target: Vector3) -> f32 {
                    ((pos.x - target.x).powi(2)
                        + (pos.y - target.y).powi(2)
                        + (pos.z - target.z).powi(2))
                    .sqrt()
                }

                for origin in self
                    .nav_grid
                    .read()
                    .iter()
                    .map(|point| {
                        Vector3::new(point[0] as f32, point[2] as f32, point[1] as f32)
                            * Vector3::new(cell_size, cell_size, cell_size)
                    })
                    .filter(|pos| distance3(*pos, origin) < 500.)
                {
                    let half_cube = cell_size / 2.;
                    unsafe {
                        // debug.AddLineOverlay(
                        //     &(Vector3::new(-half_cube, -half_cube, -half_cube) + origin),
                        //     &(Vector3::new(half_cube, half_cube, half_cube) + origin),
                        //     255,
                        //     20,
                        //     20,
                        //     false,
                        //     0.1,
                        // );
                        debug.AddBoxOverlay(
                            &origin,
                            &Vector3::new(-half_cube, -half_cube, -half_cube),
                            &Vector3::new(half_cube, half_cube, half_cube),
                            &Vector3::new(0., 0., 0.),
                            0,
                            0,
                            200,
                            255,
                            false,
                            0.1,
                        );
                    };
                }
            }
        }
    }
}

entry!(OctBots);
