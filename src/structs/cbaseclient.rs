#![allow(dead_code)]

use std::{
    ffi::{c_char, CStr},
    mem,
    ptr::addr_of,
};

use super::{bindings::CBaseClientUnion, Void};
use crate::native_types::SignonState;

pub type CbaseClientPtr = Void;

pub struct CbaseClient {
    inner: &'static CBaseClientUnion,
}

impl std::fmt::Debug for CbaseClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CbaseClient")
            .field("inner", &"can't Format")
            .finish()
    }
}

impl CbaseClient {
    pub const REALSIZE: usize = 0x2D728;

    pub fn new(ptr: CbaseClientPtr) -> Self {
        Self {
            inner: unsafe {
                mem::transmute::<_, *mut CBaseClientUnion>(ptr)
                    .as_ref()
                    .expect("this should have been checked before")
            },
        }
    }

    pub fn get_edict(&self) -> u16 {
        unsafe { self.inner.edict.edict }
    }

    pub fn is_fake_player(&self) -> bool {
        unsafe { self.inner.m_b_fake_player.m_bFakePlayer }
    }

    fn get_name_ptr(&self) -> &[c_char; 64] {
        unsafe { &self.inner.m_name.m_Name }
    }

    pub fn get_signon(&self) -> SignonState {
        unsafe { SignonState::from(self.inner.m_signon.m_Signon) }
    }

    pub fn get_name(&self) -> String {
        unsafe {
            CStr::from_ptr(mem::transmute(self.get_name_ptr() as *const [c_char; 64]))
                .to_string_lossy()
                .to_string()
        }
    }

    pub fn get_addr(&self) -> usize {
        addr_of!(*self.inner) as usize
    }

    pub fn peak(&self) {
        let name = self.get_name();

        log::info!("info about client {name}");
        log::info!("addr {:?}", self.get_addr());

        log::info!("edict : {}", self.get_edict());
        log::info!("name : {}", name);
        log::info!("signon : {:?}", self.get_signon());
        log::info!("bot : {}", self.is_fake_player());
    }
}
