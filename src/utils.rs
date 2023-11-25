use rrplug::bindings::cvar::command::COMMAND_COMPLETION_MAXITEMS;
use rrplug::bindings::cvar::{command::COMMAND_COMPLETION_ITEM_LENGTH, convar::Color};
use rrplug::prelude::*;
use std::{
    ffi::{c_char, c_void, CStr},
    marker::PhantomData,
};

use windows_sys::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory,
    LibraryLoader::{GetModuleHandleA, GetProcAddress},
    Threading::GetCurrentProcess,
};

use crate::interfaces::ENGINE_INTERFACES;

pub struct Pointer<'a, T> {
    pub ptr: *const T,
    marker: PhantomData<&'a T>,
}

pub struct CommandCompletion<'a> {
    suggestions: &'a mut [[i8; COMMAND_COMPLETION_ITEM_LENGTH as usize]],
    suggestions_left: u32,
}

pub struct CurrentCommand<'a> {
    pub cmd: &'a str,
    pub partial: &'a str,
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
impl<'a> From<*mut [c_char; COMMAND_COMPLETION_ITEM_LENGTH as usize]> for CommandCompletion<'a> {
    fn from(commands: *mut [c_char; COMMAND_COMPLETION_ITEM_LENGTH as usize]) -> Self {
        Self {
            suggestions: unsafe {
                std::slice::from_raw_parts_mut(commands, COMMAND_COMPLETION_MAXITEMS as usize)
            },
            suggestions_left: COMMAND_COMPLETION_MAXITEMS,
        }
    }
}
impl CommandCompletion<'_> {
    pub fn push<'b>(&mut self, new: &'b str) -> Result<(), &'b str> {
        if self.suggestions_left == 0 {
            return Err(new);
        }

        unsafe {
            set_c_char_array(
                &mut self.suggestions
                    [(COMMAND_COMPLETION_MAXITEMS - self.suggestions_left) as usize],
                new,
            )
        };
        self.suggestions_left -= 1;

        Ok(())
    }

    pub fn commands_used(&self) -> i32 {
        (COMMAND_COMPLETION_MAXITEMS - self.suggestions_left) as i32
    }
}

impl CurrentCommand<'_> {
    pub fn new(partial: *const c_char) -> Option<Self> {
        let partial = unsafe { CStr::from_ptr(partial).to_str() }.ok()?;
        let (name, cmd) = partial.split_once(' ').unwrap_or((partial, ""));

        Some(Self {
            cmd: name,
            partial: cmd,
        })
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

#[allow(unused)]
pub(crate) unsafe fn draw_line(
    v1: &Vector3,
    v2: &Vector3,
    color: Color,
    depthtest: bool,
    duration: f32,
) {
    let line_overlay: unsafe extern "C" fn(
        *const (),
        *const Vector3,
        *const Vector3,
        i32,
        i32,
        i32,
        i32,
        bool,
        f32,
    ) = std::mem::transmute(
        ENGINE_INTERFACES
            .get()
            .unwrap_unchecked()
            .debug_overlay
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()[4],
    );

    line_overlay(
        std::ptr::null(),
        v1,
        v2,
        color._color[0] as i32,
        color._color[1] as i32,
        color._color[2] as i32,
        color._color[3] as i32,
        depthtest,
        duration,
    )
}

pub(crate) unsafe fn client_command(edict: u16, command: *const c_char) {
    const ZERO_STRING: *const c_char = "\0".as_ptr() as *const _;

    let func = ENGINE_INTERFACES
        .get()
        .unwrap_unchecked()
        .engine_server
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()[23];
    let client_command: unsafe extern "C" fn(
        *const c_void,
        *const u16,
        *const c_char,
        *const c_char,
    ) = std::mem::transmute(func);

    client_command(std::ptr::null(), &edict, command, ZERO_STRING);
}

#[allow(unused)]
pub(crate) unsafe fn server_command(command: *const c_char) {
    let func = ENGINE_INTERFACES
        .get()
        .unwrap_unchecked()
        .engine_server
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()[21];
    let client_command: unsafe extern "C" fn(*const c_void, *const c_char) =
        std::mem::transmute(func);

    client_command(std::ptr::null(), command);
}

pub(crate) unsafe fn create_source_interface<T>(
    module: *const c_char,
    interface: *const c_char,
) -> Option<&'static mut T> {
    const CREATEINTERFACE: *const u8 = "CreateInterface\0".as_ptr() as *const _;

    let module = GetModuleHandleA(module as *const _);

    if module == 0 {
        log::error!("failed to get interface");
        return None;
    }

    let create_interface_func: extern "C" fn(*const c_char, *const i32) -> *mut T =
        std::mem::transmute(GetProcAddress(module, CREATEINTERFACE));

    return create_interface_func(interface, std::ptr::null()).as_mut();
}
