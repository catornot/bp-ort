#![allow(dead_code)]

use std::ffi::{c_char, CStr};

use crate::native_types::SignonState;

use super::Void;

pub type CbaseClientPtr = Void;

macro_rules! gen_offset_get_func {
    ($v: vis, $name: ident, $dst: ty, $offset: literal) => {
        $v fn $name(&self) -> $dst {
            unsafe { *((self.inner as usize + $offset) as *const $dst) }
        }
    };
    ($v: vis,$name: ident, ptr $dst: ty, $offset: literal) => {
        $v fn $name(&self) -> *const  $dst {
            (self.inner as usize + $offset) as *const $dst
        }
    }
}

#[derive(Debug)]
pub struct CbaseClient {
    inner: CbaseClientPtr,
}

// 0x14 is edict
// 0x16 is m_Name
// 0x2A0 is m_Signon
// 0x484 is m_bFakePlayer

impl CbaseClient {
    pub const REALSIZE: usize = 0x2D728;

    pub fn new(ptr: CbaseClientPtr) -> Self {
        Self { inner: ptr }
    }

    gen_offset_get_func!(pub, get_edict, u16, 0x14);
    gen_offset_get_func!(pub, get_name_ptr_array, ptr [c_char; 64], 0x16);
    gen_offset_get_func!(pub, get_name_ptr, ptr c_char, 0x16);
    gen_offset_get_func!(pub(self), get_signon_int, i32, 0x2A0);
    // convars here
    gen_offset_get_func!(pub, get_clan_tag_ptr, ptr [c_char; 16], 0x358);
    gen_offset_get_func!(pub, is_fake_player, bool, 0x484);
    // persistence ready here
    // persistence buffer here
    gen_offset_get_func!(pub, get_uid_ptr, ptr [c_char; 32], 0xF500);

    pub fn get_name(&self) -> String {
        unsafe {
            CStr::from_ptr(self.get_name_ptr())
                .to_string_lossy()
                .to_string()
        }
    }

    pub fn get_name_alt(&self) -> String {
        let array = unsafe { *(self.get_name_ptr_array() as *const [u8; 64]) };
        let name = String::from_utf8_lossy(array.as_ref()).to_string();

        match name.split_once('\0') {
            Some(s) => s.0.to_string(),
            None => name,
        }
    }

    pub fn get_signon(&self) -> SignonState {
        SignonState::from(self.get_signon_int())
    }
}
