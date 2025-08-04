use libc::c_char;
use parking_lot::RwLock;
use shared::plugin_interfaces::SimulationFunc;
use std::{collections::HashMap, ffi::CStr};

#[derive(Debug)]
pub struct ExternalSimulations {
    pub simulations: RwLock<HashMap<String, SimluationInfo>>,
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct SimluationInfo {
    pub name: String,
    pub simulation_maps: HashMap<usize, SimulationFunc>,
}

#[rrplug::as_interface]
impl ExternalSimulations {
    pub fn new() -> Self {
        Self {
            simulations: RwLock::new(HashMap::new()),
        }
    }

    fn rust_version_hash(&self) -> u64 {
        shared::plugin_interfaces::rust_version_hash()
    }

    fn register_simulation(
        &self,
        dll_name: *const c_char,
        simtype: usize,
        func: SimulationFunc,
    ) -> bool {
        let dll_name = unsafe { CStr::from_ptr(dll_name) }
            .to_string_lossy()
            .to_string();

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        log::info!("{dll_name} registered a simulation function for {simtype}");

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
            })
            .simulation_maps
            .insert(simtype, func)
            .is_none()
    }

    fn unregister_simulation(&self, dll_name: *const c_char, simtype: usize) -> bool {
        let dll_name = unsafe { CStr::from_ptr(dll_name) }
            .to_string_lossy()
            .to_string();

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
            })
            .simulation_maps
            .remove(&simtype)
            .is_some()
    }

    fn drop_simulation(&self, dll_name: *const c_char) -> bool {
        let dll_name = unsafe { CStr::from_ptr(dll_name) }
            .to_string_lossy()
            .to_string();

        let Some(mut simulations) = self.simulations.try_write() else {
            return false;
        };

        simulations.remove(&dll_name).is_some()
    }
}
