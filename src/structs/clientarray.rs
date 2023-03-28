use std::{
    ffi::{c_char, CStr},
    mem,
};

use crate::structs::{
    bindings::CBaseClientUnion,
    cbaseclient::{CbaseClient, CbaseClientPtr},
};

pub type ClientArrayPtr = *const [CBaseClientUnion; ClientArray::MAXCLIENTS];

pub struct ClientArray {
    inner: &'static [CBaseClientUnion; ClientArray::MAXCLIENTS],
    index: usize,
}

impl std::fmt::Debug for ClientArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CbaseClient")
            .field(
                "inner",
                &self
                    .inner
                    .iter()
                    .map(|c| unsafe {
                        CStr::from_ptr(mem::transmute(&c.name.m_Name as *const [c_char; 64]))
                            .to_string_lossy()
                            .to_string()
                    })
                    .collect::<Vec<String>>(),
            )
            .field("index", &self.index)
            .finish()
    }
}

impl ClientArray {
    pub const MAXCLIENTS: usize = 32;

    pub fn new(ptr: ClientArrayPtr) -> Self {
        Self {
            inner: unsafe { ptr.as_ref() }.expect("how tf"),
            index: 0,
        }
    }

    pub fn reset_iter(&mut self) {
        self.index = 0;
    }

    pub fn peak_array(&mut self) {
        self.reset_iter();

        for (i, client) in self.enumerate() {
            log::info!("info about client {i}");
            log::info!("addr {:?}", client.get_addr() as *const CbaseClient);
            log::info!("size of ptr : {:?}", std::mem::size_of::<CbaseClientPtr>());
            
            log::info!("edict : {}", client.get_edict());
            log::info!("name : {}", client.get_name());
            log::info!("signon : {:?}", client.get_signon());
            log::info!("bot : {}", client.is_fake_player());
        }
        self.reset_iter();
    }

    #[allow(dead_code)]
    pub fn get_inner_ptr(&self) -> ClientArrayPtr {
        self.inner
    }
}

impl Iterator for ClientArray {
    type Item = CbaseClient;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= Self::MAXCLIENTS {
            self.reset_iter();
            None?
        }

        let client = &self.inner[self.index];

        self.index += 1;

        Some(CbaseClient::from(client))
    }
}
