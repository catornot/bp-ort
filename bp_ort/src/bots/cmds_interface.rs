use libc::c_char;
use parking_lot::RwLock;
use rrplug::high::{UnsafeHandle, vector::Vector3};
use shared::{
    bindings::ENGINE_FUNCTIONS,
    plugin_interfaces::{
        Array, BotInitFunction, CURERENT_INTERFACE_VERSION, PreSimulateFunction, SimulationFunc,
    },
    utils::iterate_c_array_sized,
};
use std::{collections::HashMap, ffi::CStr};

use crate::navmesh::{Hull, RECAST_DETOUR, navigation::Navigation};

#[derive(Debug)]
pub struct ExternalSimulations {
    pub simulations: RwLock<HashMap<String, SimluationInfo>>,
    pub navmeshes: RwLock<HashMap<Hull, UnsafeHandle<Navigation>>>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SimluationInfo {
    pub name: String,
    pub simulation_maps: HashMap<usize, SimulationFunc>,
    pub init_func: Option<BotInitFunction>,
    pub pre_simulate: Option<PreSimulateFunction>,
}

#[rrplug::as_interface]
impl ExternalSimulations {
    pub fn new() -> Self {
        Self {
            simulations: RwLock::new(HashMap::new()),
            navmeshes: RwLock::new(HashMap::new()),
        }
    }

    fn rust_version_hash(&self) -> u64 {
        shared::plugin_interfaces::rust_version_hash()
    }

    fn interface_version(&self) -> u64 {
        CURERENT_INTERFACE_VERSION
    }

    fn register_simulation(
        &self,
        dll_name: *const c_char,
        simtype: usize,
        func: SimulationFunc,
    ) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        if simtype < 40 {
            log::error!("{simtype} wasn't accepted; <= 40 is reserved for bp_ort");
            return false;
        }

        log::info!("{dll_name} registered a simulation function for {simtype}");

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
                pre_simulate: None,
            })
            .simulation_maps
            .insert(simtype, func)
            .is_none()
    }

    fn unregister_simulation(&self, dll_name: *const c_char, simtype: usize) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
                pre_simulate: None,
            })
            .simulation_maps
            .remove(&simtype)
            .is_some()
    }

    fn drop_simulation(&self, dll_name: *const c_char) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        simulations.remove(&dll_name).is_some()
    }

    pub fn set_bot_init(&self, dll_name: *const c_char, func: BotInitFunction) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        if let Some(engine_functions) = ENGINE_FUNCTIONS.get()
            && !simulations.contains_key(&dll_name)
        {
            for client in
                unsafe { iterate_c_array_sized::<_, 32>(engine_functions.client_array.into()) }
                    .filter(|client| client.m_bFakePlayer)
            {
                func(client.m_nHandle - 1, client)
            }
        }

        log::info!("{dll_name} registered a bot init function");

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
                pre_simulate: None,
            })
            .init_func
            .replace(func);

        true
    }

    pub fn unregister_bot_init(&self, dll_name: *const c_char) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
                pre_simulate: None,
            })
            .init_func
            .take();

        true
    }

    pub fn register_pre_simulation(
        &self,
        dll_name: *const c_char,
        pre_simulation: PreSimulateFunction,
    ) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        log::info!("{dll_name} registered a pre simulation function");

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
                pre_simulate: None,
            })
            .pre_simulate
            .replace(pre_simulation);

        true
    }

    pub fn unregister_pre_simulation(&self, dll_name: *const c_char) -> bool {
        let dll_name = cstring_to_string(dll_name);

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
                pre_simulate: None,
            })
            .pre_simulate
            .take();

        true
    }

    pub fn find_path(&self, hull: Hull, start: Vector3, end: Vector3) -> Array<Vector3> {
        let Some(mut navmeshes) = self.navmeshes.try_write() else {
            return Array::new(&[]);
        };

        if navmeshes.get(&hull).is_none() {
            if let Some(navigation) = Navigation::new(hull) {
                navmeshes
                    .entry(hull)
                    .or_insert(unsafe { UnsafeHandle::new(navigation) });
            } else {
                return Array::new(&[]);
            }
        }
        let navigation = navmeshes
            .get_mut(&hull)
            .expect("this should have never happened");

        _ = navigation
            .get_mut()
            .new_path(start, end, RECAST_DETOUR.wait());
        Array::new(navigation.get_mut().path_points.as_slice())
    }

    pub fn find_random_point(
        &self,
        hull: Hull,
        center: Vector3,
        max_radius: f32,
        min_radius: f32,
    ) -> Vector3 {
        let Some(mut navmeshes) = self.navmeshes.try_write() else {
            return center;
        };

        if navmeshes.get(&hull).is_none() {
            if let Some(navigation) = Navigation::new(hull) {
                navmeshes
                    .entry(hull)
                    .or_insert(unsafe { UnsafeHandle::new(navigation) });
            } else {
                return center;
            }
        }
        let navigation = navmeshes
            .get_mut(&hull)
            .expect("this should have never happened");

        navigation
            .get_mut()
            .random_point_around(
                center,
                max_radius,
                (min_radius < max_radius && min_radius >= 0.).then_some(min_radius),
            )
            .unwrap_or(center)
    }
}

pub fn cstring_to_string(string: *const c_char) -> String {
    unsafe { CStr::from_ptr(string) }
        .to_string_lossy()
        .to_string()
}
