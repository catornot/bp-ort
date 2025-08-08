use rrplug::{
    bindings::class_types::{cbaseentity::CBaseEntity, cplayer::CPlayer},
    mid::utils::try_cstring,
};
use std::{
    ffi::{c_char, c_void, CStr},
    marker::PhantomData,
};

use windows_sys::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory, Threading::GetCurrentProcess,
};

use crate::bindings::{EngineFunctions, ServerFunctions, ENGINE_FUNCTIONS};

pub struct ClassNameIter<'a> {
    // class_name: &'a CStr,
    magic_class_name: *const i8,
    server_funcs: &'a ServerFunctions,
    ent: *mut CBaseEntity,
}

impl<'a> ClassNameIter<'a> {
    pub fn new(class_name: &'a CStr, server_funcs: &'a ServerFunctions) -> Self {
        let mut magic = std::ptr::null();

        unsafe {
            (server_funcs.some_magic_function_for_class_name)(&mut magic, class_name.as_ptr())
        };

        ClassNameIter {
            server_funcs,
            ent: std::ptr::null_mut(),
            magic_class_name: magic,
        }
    }
}

impl Iterator for ClassNameIter<'_> {
    type Item = *mut CBaseEntity;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            (self.server_funcs.find_next_entity_by_class_name)(
                self.server_funcs.ent_list.cast(),
                self.ent,
                self.magic_class_name,
            )
            .as_mut()
            .inspect(|ent| self.ent = std::ptr::from_ref(*ent).cast_mut())
            .map(std::ptr::from_mut)
        }
    }
}

pub struct Pointer<'a, T> {
    pub ptr: *const T,
    marker: PhantomData<&'a T>,
}

impl<T> From<*const T> for Pointer<'_, T> {
    fn from(value: *const T) -> Self {
        Self {
            ptr: value,
            marker: PhantomData,
        }
    }
}

impl<T> From<*mut T> for Pointer<'_, T> {
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
pub unsafe fn iterate_c_array_sized<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &T> {
    let ptr: *const T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_ref())
}

#[inline]
pub unsafe fn iterate_c_array_sized_mut<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &mut T> {
    let ptr: *mut T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_mut())
}

#[inline]
pub unsafe fn set_c_char_array<const U: usize>(buf: &mut [c_char; U], new: &str) {
    *buf = [0; U]; // null everything
    buf.iter_mut()
        .zip(new.as_bytes())
        .for_each(|(buf_char, new)| *buf_char = *new as i8);
    buf[U - 1] = 0; // also null last byte
}

#[inline]
pub fn get_c_char_array_lossy<const U: usize>(buf: &[c_char; U]) -> String {
    let index = buf
        .iter()
        .position(|c| *c == b'\0' as i8)
        .unwrap_or(buf.len());
    String::from_utf8_lossy(&buf.map(|i| i as u8)[0..index]).to_string()
}

#[inline]
pub fn get_c_char_array<const U: usize>(buf: &[i8; U]) -> Option<&str> {
    let index = buf
        .iter()
        .position(|c| *c == b'\0' as i8)
        .unwrap_or(buf.len());
    // SAFETY: an i8 is a valid u8
    str::from_utf8(&(unsafe { std::mem::transmute::<&[i8; U], &[u8; U]>(buf) })[0..index]).ok()
}

#[inline]
pub unsafe fn from_c_string<T: From<String>>(ptr: *const c_char) -> T {
    CStr::from_ptr(ptr).to_string_lossy().to_string().into()
}

#[allow(unused)]
#[inline]
pub unsafe fn patch(addr: usize, bytes: &[u8]) {
    WriteProcessMemory(
        GetCurrentProcess(),
        addr as *const c_void,
        bytes as *const _ as *const c_void,
        bytes.len(),
        std::ptr::null_mut(),
    );
}

pub fn send_client_print(player: &CPlayer, msg: &str) -> Option<()> {
    let engine = ENGINE_FUNCTIONS.wait();

    let client = unsafe {
        engine
            .client_array
            .add((player.pl.index as usize).checked_sub(1)?)
            .as_ref()?
    };
    let msg = try_cstring(msg).ok()?;

    unsafe { (engine.cgame_client_printf)(client, msg.as_ptr()) };

    None
}

pub fn lookup_ent(handle: i32, server_funcs: &ServerFunctions) -> Option<&CBaseEntity> {
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
    lookup_ent(player.m_playerScriptNetDataGlobal, server_funcs)
        .map(|ent| unsafe {
            (server_funcs.get_net_var_from_ent)(ent, netvar.as_ptr(), index, buf.as_mut_ptr())
        })
        .map(|_| buf[0])
}

pub fn get_ents_by_class_name<'a>(
    name: &'a CStr,
    server_funcs: &'a ServerFunctions,
) -> impl Iterator<Item = *mut CBaseEntity> + 'a {
    ClassNameIter::new(name, server_funcs)
}

pub fn get_weaponx_name<'a>(
    weapon: &'a CBaseEntity,
    server_funcs: &ServerFunctions,
    engine_funcs: &EngineFunctions,
) -> Option<&'a str> {
    unsafe {
        rrplug::mid::utils::str_from_char_ptr((engine_funcs
            .cnetwork_string_table_vtable
            .as_ref()?
            .get_string)(
            *server_funcs.weapon_names_string_table.cast(),
            *std::ptr::from_ref(weapon).cast::<i32>().byte_offset(0x12d8), // this is the name index TODO: make this a actual field in some struct
        ))
    }
}
