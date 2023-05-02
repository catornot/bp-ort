use std::{ffi::c_void, mem};

pub type FuncHooks<'a> = Vec<&'a dyn Hooks>;

pub struct DllHook {
    dll_ptr: *const c_void,
}

impl DllHook {
    pub fn new(dll_ptr: *const c_void) -> Self {
        Self { dll_ptr }
    }

    pub fn offset<T>(&self) -> *const T {
        unsafe { mem::transmute(self.dll_ptr) }
    }

    pub fn get_ptr(&self) -> *const c_void {
        self.dll_ptr
    }
}

#[allow(unused_variables)]
pub trait Hooks {
    fn hook_client(&self, dll: &DllHook) {}
    fn hook_server(&self, dll: &DllHook) {}
    fn hook_engine(&self, dll: &DllHook) {}
    fn hook_matsys(&self, dll: &DllHook) {}
}
