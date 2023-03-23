use std::{
    mem::{self},
    ptr::addr_of,
};

use crate::structs::cbaseclient::{CbaseClient, CbaseClientPtr};

use super::Void;

// pub type ClientArrayPtr = *const [Void; ClientArray::MAXCLIENTS];
pub type ClientArrayPtr = Void;

#[derive(Debug)]
pub struct ClientArray {
    inner: ClientArrayPtr,
    index: usize,
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
            log::info!("addr {client:?}");

            let client = CbaseClient::new(client);

            log::info!("edict : {}", client.get_edict());
            log::info!("name alt : {}", client.get_name_alt());
            log::info!("name : {}", client.get_name());
            log::info!("signon : {:?}", client.get_signon());
            log::info!("bot : {}", client.is_fake_player());
        }

        for client_addr in
            unsafe { *mem::transmute::<_, *const [Void; Self::MAXCLIENTS]>(self.inner) }
        {
            log::info!("v2 addr {client_addr:?}");
        }

        self.reset_iter();
    }

    #[allow(dead_code)]
    pub fn get_inner_ptr(&self) -> ClientArrayPtr {
        self.inner
    }
}

impl Iterator for ClientArray {
    type Item = CbaseClientPtr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= Self::MAXCLIENTS {
            self.reset_iter();
            return None;
        }

        // let client = (addr_of!(self.inner) as usize + size_of::<ClientArrayPtr>() * self.index)
        //     as CbaseClientPtr;

        let client = (unsafe { addr_of!(self.inner).add(self.index) }) as CbaseClientPtr;

        // let client = unsafe { (*self.inner)[self.index] };

        self.index += 1;

        if client.is_null() {
            return None;
        }

        Some(client)
    }

    // fn find<P>(&mut self, predicate: P) -> Option<Self::Item>
    //     where
    //         Self: Sized,
    //         P: FnMut(&Self::Item) -> bool, {

    // }
}
