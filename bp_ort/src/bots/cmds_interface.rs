use libc::c_char;
use parking_lot::RwLock;
use shared::{
    bindings::ENGINE_FUNCTIONS,
    plugin_interfaces::{BotInitFunction, SimulationFunc},
    utils::iterate_c_array_sized,
};
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
    pub init_func: Option<BotInitFunction>,
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
                func(client.m_nHandle, client)
            }
        }

        simulations
            .entry(dll_name.clone())
            .or_insert_with(|| SimluationInfo {
                name: dll_name,
                simulation_maps: Default::default(),
                init_func: None,
            })
            .init_func
            .replace(func);

        true
    }
    pub fn drop_bot_init(&self, dll_name: *const c_char) -> bool {
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
            })
            .init_func
            .take();

        true
    }
}

pub fn cstring_to_string(string: *const c_char) -> String {
    unsafe { CStr::from_ptr(string) }
        .to_string_lossy()
        .to_string()
}
