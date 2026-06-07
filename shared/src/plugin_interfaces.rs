use crate::{bindings::CUserCmd, cmds_helper::CUserCmdHelper};
use rrplug::{
    bindings::class_types::{client::CClient, cplayer::CPlayer},
    create_external_interface,
    high::vector::Vector3,
    mid::source_alloc::SOURCE_ALLOC,
};
use std::{ffi::c_char, ptr::NonNull};

pub type SimulationFunc = extern "C" fn(&CUserCmdHelper, &mut CPlayer) -> CUserCmd;

pub type BotInitFunction = extern "C" fn(u16, &CClient);

pub type PreSimulateFunction = extern "C" fn(bool);

pub static CURERENT_INTERFACE_VERSION: u64 = 0;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hull {
    Human,
    Medium,
    FlyingVehicle,
    Small,
    Titan,
}

#[repr(C)]
pub struct Array<T> {
    pub ptr: NonNull<T>,
    pub len: usize,
}

impl<T: Clone> Array<T> {
    pub fn new(slice: &[T]) -> Self {
        let ptr = unsafe {
            let buf = SOURCE_ALLOC
                .get_underlying_alloc()
                .Alloc(std::mem::size_of_val(slice))
                .cast_mut()
                .cast::<T>();

            for (i, elem) in slice.iter().enumerate() {
                *buf.add(i) = elem.clone();
            }

            NonNull::new(buf).expect("we would have already crashed")
        };

        Self {
            ptr,
            len: slice.len(),
        }
    }
}

impl<T> Array<T> {
    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for Array<T> {
    fn drop(&mut self) {
        unsafe {
            SOURCE_ALLOC
                .get_underlying_alloc()
                .Free(self.ptr.as_ptr().cast())
        };
    }
}

create_external_interface! {
    pub ExternalSimulations + ExternalSimulations001 => {
        pub fn rust_version_hash() -> u64;
        pub fn interface_version() -> u64;
        pub fn register_simulation(dll_name: *const c_char, simtype: usize, func: SimulationFunc) -> bool;
        pub fn unregister_simulation(dll_name: *const c_char, simtype: usize) -> bool;
        pub fn drop_simulation(dll_name: *const c_char) -> bool;
        pub fn set_bot_init(dll_name: *const c_char, func: BotInitFunction) -> bool;
        pub fn drop_bot_init(dll_name: *const c_char) -> bool;
        pub fn register_pre_simulate(dll_name: *const c_char, func: PreSimulateFunction) -> bool;
        pub fn unregister_pre_simulate(dll_name: *const c_char) -> bool;
        pub fn find_path(hull: Hull, start: Vector3, end: Vector3) -> Array<Vector3>;
        pub fn find_random_point(hull: Hull, center: Vector3, max_radius: f32, min_radius: Option<f32>) -> Vector3;
    }
}

pub fn rust_version_hash() -> u64 {
    include!(concat!(env!("OUT_DIR"), "/rustc"))
}

unsafe impl Sync for ExternalSimulations {}
unsafe impl Send for ExternalSimulations {}
