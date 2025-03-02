use std::{ffi::CStr, mem::MaybeUninit, ops::Range};

use once_cell::sync::OnceCell;
use parking_lot::{Mutex, RwLock};
use rrplug::{exports::windows::Win32::Foundation::HMODULE, prelude::*};
use shared::{
    bindings::{
        CTraceFilterSimple, ClientFunctions, EngineFunctions, HostState, MatSysFunctions, Ray,
        ServerFunctions, TraceResults, CLIENT_FUNCTIONS, ENGINE_FUNCTIONS, MATSYS_FUNCTIONS,
        SERVER_FUNCTIONS,
    },
    interfaces::{IVDebugOverlay, IVEngineServer},
};

const NAV_CUBE_SIZE: usize = 50;
const HALF_CUBE: f32 = NAV_CUBE_SIZE as f32 / 2.;

pub static ENGINE_INTERFACES: OnceCell<EngineInterfaces> = OnceCell::new();

pub struct EngineInterfaces {
    pub debug_overlay: &'static IVDebugOverlay, // since it's a ptr to class which has a ptr to vtable
    pub engine_server: &'static IVEngineServer,
}

unsafe impl Sync for EngineInterfaces {}
unsafe impl Send for EngineInterfaces {}

pub struct OctBots {
    #[allow(clippy::type_complexity)]
    nav_grid: RwLock<Vec<Vec<Vec<(bool, Vector3)>>>>,
    current_map: Mutex<String>,
}

impl Plugin for OctBots {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"octbots", c" OCTBOTS ", c"OCTBOTS", PluginContext::all());

    fn new(_reloaded: bool) -> Self {
        Self {
            nav_grid: RwLock::new(Vec::new()),
            current_map: Mutex::new("".to_string()),
        }
    }

    fn plugins_loaded(&self, _engine_token: EngineToken) {}

    fn on_dll_load(
        &self,
        engine: Option<&EngineData>,
        dll_ptr: &DLLPointer,
        engine_token: EngineToken,
    ) {
        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        let Some(engine_data) = engine else { return };

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

        engine_data
            .register_concommand("test_raycasting", test_raycasting, "", 0, engine_token)
            .expect("couldn't register a concommand");
    }

    fn on_reload_request(&self) -> reloading::ReloadResponse {
        // has to be reloadable
        unsafe { reloading::ReloadResponse::allow_reload() }
    }

    #[allow(unused_variables, unreachable_code)]
    fn runframe(&self, _engine_token: EngineToken) {
        return;

        if let Some(state) = unsafe { ENGINE_FUNCTIONS.wait().host_state.as_ref() } {
            let current_name =
                unsafe { CStr::from_ptr(state.level_name.as_ptr()).to_string_lossy() };
            let mut load_nav = self.current_map.lock();

            if *load_nav != current_name && state.current_state == HostState::Run {
                *self.nav_grid.write() = generate_nav_data((-20..20, -20..20, -20..20));

                *load_nav = current_name.to_string();
            } else {
                let debug = ENGINE_INTERFACES.wait().debug_overlay;
                return;

                log::info!("wow");

                for (exists, origin) in self
                    .nav_grid
                    .read()
                    .iter()
                    .flat_map(|vec| vec.iter())
                    .flat_map(|vec| vec.iter())
                    .copied()
                {
                    if !exists {
                        continue;
                    }

                    unsafe {
                        debug.AddLineOverlay(
                            &(Vector3::new(-HALF_CUBE, -HALF_CUBE, -HALF_CUBE) + origin),
                            &(Vector3::new(HALF_CUBE, HALF_CUBE, HALF_CUBE) + origin),
                            255,
                            20,
                            20,
                            false,
                            0.1,
                        );
                        debug.AddBoxOverlay(
                            &origin,
                            &Vector3::new(-HALF_CUBE, -HALF_CUBE, -HALF_CUBE),
                            &Vector3::new(HALF_CUBE, HALF_CUBE, HALF_CUBE),
                            &Vector3::new(0., 0., 0.),
                            20,
                            20,
                            20,
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

#[rrplug::concommand]
fn test_raycasting() {
    for x_range in (-12..12).map(|i| i * 34..(i + 1) * 34) {
        log::info!("thread for {x_range:?}");
        generate_nav_data((x_range, -400..400, -400..400));
    }
}

fn generate_nav_data(
    portion: (Range<i32>, Range<i32>, Range<i32>),
) -> Vec<Vec<Vec<(bool, Vector3)>>> {
    let engine_funcs = ENGINE_FUNCTIONS.wait();
    let server_funcs = SERVER_FUNCTIONS.wait();

    portion
        .clone()
        .0
        .map(|x| {
            log::info!("raycasting slice {x}");
            portion
                .clone()
                .1
                .map(|y| {
                    portion
                        .clone()
                        .2
                        .map(move |z| (x, y, z))
                        .map(|(x, y, z)| {
                            Vector3::new(
                                x as f32 * NAV_CUBE_SIZE as f32,
                                y as f32 * NAV_CUBE_SIZE as f32,
                                z as f32 * NAV_CUBE_SIZE as f32,
                            )
                        })
                        .map(|origin| (check_block(engine_funcs, server_funcs, origin), origin))
                        .collect()
                })
                .collect()
        })
        .collect()
}

fn check_block(
    engine_funcs: &EngineFunctions,
    server_funcs: &ServerFunctions,
    origin: Vector3,
) -> bool {
    const TRACE_MASK_SHOT: i32 = 1178615859;
    // const TRACE_MASK_SOLID_BRUSHONLY: i32 = 16907;
    const TRACE_COLLISION_GROUP_BLOCK_WEAPONS: i32 = 0x12; // 18

    let mut result: MaybeUninit<TraceResults> = MaybeUninit::zeroed();
    let mut ray = unsafe {
        let mut ray: Ray = MaybeUninit::zeroed().assume_init(); // all zeros is correct for Ray
        ray.unk6 = 0;
        (server_funcs.create_trace_hull)(
            &mut ray,
            &Vector3::new(origin.x, origin.y, origin.z + HALF_CUBE - 1.),
            &Vector3::new(origin.x, origin.y, origin.z - HALF_CUBE + 1.),
            &Vector3::new(-HALF_CUBE, -HALF_CUBE, -1.),
            &Vector3::new(HALF_CUBE, HALF_CUBE, 1.),
        );
        ray
    };
    let filter: *const CTraceFilterSimple = &CTraceFilterSimple {
        vtable: server_funcs.simple_filter_vtable,
        unk: 0,
        pass_ent: std::ptr::null(),
        should_hit_func: std::ptr::null(),
        collision_group: TRACE_COLLISION_GROUP_BLOCK_WEAPONS,
    };

    ray.is_smth = false;
    unsafe {
        (engine_funcs.trace_ray_filter)(
            (*server_funcs.ctraceengine) as *const libc::c_void,
            &ray,
            TRACE_MASK_SHOT as u32,
            filter.cast(),
            // std::ptr::null(),
            result.as_mut_ptr(),
        );
    }

    let result = unsafe { result.assume_init() };

    !result.start_solid && result.fraction_left_solid == 0.0
}
