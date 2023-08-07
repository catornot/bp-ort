use std::marker::PhantomData;

use rrplug::prelude::*;

mod bindings;
mod bots;

use crate::{
    bindings::{
        ClientFunctions, EngineFunctions, MatSysFunctions, ServerFunctions, CLIENT_FUNCTIONS,
        ENGINE_FUNCTIONS, MATSYS_FUNCTIONS, SERVER_FUNCTIONS,
    },
    bots::Bots,
    screen_detour::hook_materialsystem,
};

// todo put these into folders

mod screen_detour;

#[derive(Debug)]
pub struct HooksPlugin {
    pub bots: Bots,
}

impl Plugin for HooksPlugin {
    fn new(plugin_data: &PluginData) -> Self {
        Self {
            bots: Bots::new(plugin_data),
        }
    }

    fn main(&self) {}

    fn on_dll_load(&self, engine: &PluginLoadDLL, dll_ptr: &DLLPointer) {
        self.bots.on_dll_load(engine, dll_ptr);

        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        match engine {
            PluginLoadDLL::Other(other) if other == "materialsystem_dx11.dll" => {
                hook_materialsystem(dll_ptr.get_dll_ptr())
            }
            _ => {}
        }
    }
}

entry!(HooksPlugin);

pub(crate) unsafe fn iterate_c_array_sized<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &T> {
    let ptr: *const T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_ref())
}

pub struct Pointer<'a, T> {
    pub ptr: *const T,
    marker: PhantomData<&'a T>,
}

impl<'a, T> From<*const T> for Pointer<'a, T> {
    fn from(value: *const T) -> Self {
        Self {
            ptr: value,
            marker: PhantomData,
        }
    }
}

impl<'a, T> From<*mut T> for Pointer<'a, T> {
    fn from(value: *mut T) -> Self {
        Self {
            ptr: value.cast_const(),
            marker: PhantomData,
        }
    }
}

impl<'a, T> From<Pointer<'a, T>> for *const T {
    fn from(val: Pointer<'a, T>) -> Self {
        val.ptr
    }
}

impl<'a, T> From<Pointer<'a, T>> for *mut T {
    fn from(val: Pointer<'a, T>) -> Self {
        val.ptr.cast_mut()
    }
}
