use rrplug::{bindings::class_types::cplayer::CPlayer, mid::utils::try_cstring};
use std::{
    ffi::{c_char, c_void, CStr},
    marker::PhantomData,
};

use windows_sys::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory, Threading::GetCurrentProcess,
};

use crate::bindings::{CBaseEntity, ServerFunctions, ENGINE_FUNCTIONS};

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

#[inline]
pub(crate) unsafe fn iterate_c_array_sized<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &T> {
    let ptr: *const T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_ref())
}

#[inline]
pub(crate) unsafe fn set_c_char_array<const U: usize>(buf: &mut [c_char; U], new: &str) {
    *buf = [0; U]; // null everything
    buf.iter_mut()
        .zip(new.as_bytes())
        .for_each(|(buf_char, new)| *buf_char = *new as i8);
    buf[U - 1] = 0; // also null last byte
}

#[inline]
pub(crate) unsafe fn from_c_string<T: From<String>>(ptr: *const c_char) -> T {
    CStr::from_ptr(ptr).to_string_lossy().to_string().into()
}

#[allow(unused)]
#[inline]
pub(crate) unsafe fn patch(addr: usize, bytes: &[u8]) {
    WriteProcessMemory(
        GetCurrentProcess(),
        addr as *const c_void,
        bytes as *const _ as *const c_void,
        bytes.len(),
        std::ptr::null_mut(),
    );
}

pub(crate) fn send_client_print(player: &CPlayer, msg: &str) -> Option<()> {
    let engine = ENGINE_FUNCTIONS.wait();

    let client = unsafe {
        engine
            .client_array
            .add(player.player_index.copy_inner() as usize - 1)
            .as_ref()?
    };
    let msg = try_cstring(msg).ok()?;

    unsafe { (engine.cgame_client_printf)(client, msg.as_ptr()) };

    None
}

pub(crate) fn lookup_ent(handle: i32, server_funcs: &ServerFunctions) -> Option<&CBaseEntity> {
    let entry_index = (handle & 0xffff) as usize;
    let serial_number = handle >> 0x10;

    if handle == -1
        || entry_index > 0x3fff
        || unsafe {
            server_funcs
                .ent_list
                .add(entry_index)
                .as_ref()?
                .serial_number
        } != serial_number
    {
        return None;
    }

    unsafe {
        server_funcs
            .ent_list
            .add(entry_index)
            .as_ref()?
            .ent
            .as_ref()
    }
}

pub fn get_net_var(
    player: &CPlayer,
    netvar: &CStr,
    index: i32,
    server_funcs: &ServerFunctions,
) -> Option<i32> {
    let mut buf = [0; 4];
    lookup_ent(
        unsafe { player.player_script_net_data_global.copy_inner() },
        server_funcs,
    )
    .map(|ent| unsafe {
        (server_funcs.get_net_var_from_ent)(ent, netvar.as_ptr(), index, buf.as_mut_ptr())
    })
    .map(|_| buf[0])
}
