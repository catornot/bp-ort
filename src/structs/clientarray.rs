use std::{
    ffi::{c_char, CStr},
    mem,
};

use crate::structs::{
    bindings::CBaseClientUnion,
    cbaseclient::{CbaseClient, CbaseClientPtr},
};

// use super::Void;

// pub type ClientArrayPtr = *const [Void; ClientArray::MAXCLIENTS];
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

            // let client = CbaseClient::new(client);

            log::info!("edict : {}", client.get_edict());
            // log::info!("name alt : {}", client.get_name_alt());
            log::info!("name : {}", client.get_name());
            log::info!("signon : {:?}", client.get_signon());
            log::info!("bot : {}", client.is_fake_player());
        }

        // log::info!("v2 addr {:?}", unsafe {
        //     *std::mem::transmute::<_, *const [*const CBaseClientUnion; Self::MAXCLIENTS]>(self.inner)
        // });

        // for client_ptr in (0..Self::MAXCLIENTS).map(|i| {
        //     (self.inner as usize + std::mem::size_of::<*const *const CbaseClient>() * i)
        //         as *const CbaseClient
        // }) {
        //     log::info!("v3 addr {client_ptr:?}");
        // }

        // log::info!(
        //     "v4 addr {:?}",
        //     (0..Self::MAXCLIENTS)
        //         .map(|i| (unsafe { self.inner.offset(0x4 * i as isize) }) as *const c_void)
        //         .collect::<Vec<*const c_void>>()
        // );

        // log::info!("sizeof(ClientArrayPtr) {}", unsafe {
        //     size_of_val_raw::<*const CBaseClientUnion>(self.inner.cast())
        // });

        // log::info!(
        //     "v5 addr {:?}",
        //     (0..Self::MAXCLIENTS)
        //         .map(|i| (unsafe {
        //             self.inner.offset(
        //                 (size_of_val_raw::<*const CBaseClientUnion>(self.inner.cast()) * i)
        //                     .try_into()
        //                     .unwrap(),
        //             )
        //         }) as *const c_void)
        //         .collect::<Vec<*const c_void>>()
        // );

        // log::info!("v6 addr {:?}", unsafe {
        //     **std::mem::transmute::<_, *const *const [*const c_void; Self::MAXCLIENTS]>(self.inner)
        // });

        // log::info!("v7 addr {:?}", unsafe {
        //     slice::from_raw_parts(self.inner, Self::MAXCLIENTS).iter().collect::<Vec<_>>()
        // });

        log::info!("whole thing {:?}", self);

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
