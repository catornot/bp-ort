use std::mem;

use crate::structs::{
    bindings::CBaseClientUnion,
    cbaseclient::{CbaseClient, CbaseClientPtr},
};

// use super::Void;

// pub type ClientArrayPtr = *const [Void; ClientArray::MAXCLIENTS];
pub type ClientArrayPtr = *mut CBaseClientUnion;

pub struct ClientArray {
    inner: ClientArrayPtr,
    index: usize,
}

impl std::fmt::Debug for ClientArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CbaseClient")
            .field("inner", &"can't Format")
            .field("index", &self.index)
            .finish()
    }
}

impl ClientArray {
    pub const MAXCLIENTS: usize = 32;

    pub fn new(ptr: ClientArrayPtr) -> Self {
        Self {
            inner: ptr,
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
            log::info!("addr {:?}", client.get_addr());

            // let client = CbaseClient::new(client);

            log::info!("edict : {}", client.get_edict());
            // log::info!("name alt : {}", client.get_name_alt());
            log::info!("name : {}", client.get_name());
            log::info!("signon : {:?}", client.get_signon());
            log::info!("bot : {}", client.is_fake_player());
        }

        // for client_addr in
        //     unsafe { *mem::transmute::<_, *const [Void; Self::MAXCLIENTS]>(self.inner) }
        // {
        //     log::info!("v2 addr {client_addr:?}");
        // }

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

        let client = unsafe { mem::transmute::<_, CbaseClientPtr>(self.inner.add(self.index)) };

        self.index += 1;

        CbaseClient::new(client).or_else(|| {
            self.reset_iter();
            None
        })
    }
}
