use std::{ffi::CString, fmt::Display, str::FromStr};

use rrplug::bindings::server::{client::CClient, cplayer::CPlayer};

use crate::{interfaces::IVEngineServer, utils::get_player_index};

pub trait ClientIndex {
    fn get_index(&self) -> u32;
}

#[derive(Debug)]
pub enum PersitenceSetError {
    NotConnected,
    InvalidPersistence,
    WrongType(u8),
    MissingPersistenceName,
    InvalidCString,
}

pub struct ClientPersistence {
    engine_server: &'static IVEngineServer,
    ignore_non_connected: bool,
}

unsafe impl Sync for ClientPersistence {}
unsafe impl Send for ClientPersistence {}

impl std::fmt::Debug for ClientPersistence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientPersistence").finish_non_exhaustive()
    }
}

impl ClientPersistence {
    pub fn new(engine_server: &'static IVEngineServer, ignore_non_connected: bool) -> Self {
        Self {
            engine_server,
            ignore_non_connected,
        }
    }

    pub fn set_player_loadout_persistence_string<T: ClientIndex>(
        &self,
        client: T,
        loadout_type: &str,
        loadout_index: usize,
        property_name: &str,
        value: &str,
    ) -> Result<(), PersitenceSetError> {
        self.set_persistent_string(
            client,
            &(loadout_type.to_string()
                + "Loadouts["
                + &loadout_index.to_string()
                + "]."
                + property_name),
            value,
        )
    }

    pub fn set_player_loadout_persistence_int<T: ClientIndex>(
        &self,
        client: T,
        loadout_type: &str,
        loadout_index: usize,
        property_name: &str,
        value: i32,
    ) -> Result<(), PersitenceSetError> {
        self.set_persistent_int(
            client,
            &(loadout_type.to_string()
                + "Loadouts["
                + &loadout_index.to_string()
                + "]."
                + property_name),
            value,
        )
    }

    pub fn set_persistent_string<T: ClientIndex>(
        &self,
        client: T,
        name: &str,
        value: &str,
    ) -> Result<(), PersitenceSetError> {
        self.is_valid(client.get_index())?;

        let name = CString::from_str(name).map_err(|_| PersitenceSetError::InvalidCString)?;
        let value = CString::from_str(value).map_err(|_| PersitenceSetError::InvalidCString)?;

        let mut persistence_index = 0;
        let ty = unsafe {
            self.engine_server.GetPersistenceDataType(
                client.get_index(),
                name.as_ptr(),
                &mut persistence_index,
            )
        } as u8;
        if ty == 3 {
            unsafe {
                self.engine_server.SetPersistentString(
                    client.get_index(),
                    persistence_index,
                    value.as_ptr(),
                )
            };
            Ok(())
        } else {
            Err(PersitenceSetError::WrongType(ty))
        }
    }

    pub fn set_persistent_int<T: ClientIndex>(
        &self,
        client: T,
        name: &str,
        value: i32,
    ) -> Result<(), PersitenceSetError> {
        self.is_valid(client.get_index())?;

        let name = CString::from_str(name).map_err(|_| PersitenceSetError::InvalidCString)?;

        let mut persistence_index = 0;
        let ty = unsafe {
            self.engine_server.GetPersistenceDataType(
                client.get_index(),
                name.as_ptr(),
                &mut persistence_index,
            )
        } as u8;
        if ty == 1 {
            unsafe {
                self.engine_server
                    .SetPersistentInt1(client.get_index(), persistence_index, value)
            };
            Ok(())
        } else if ty == 2 {
            unsafe {
                self.engine_server
                    .SetPersistentInt2(client.get_index(), persistence_index, value)
            };
            Ok(())
        } else {
            Err(PersitenceSetError::WrongType(ty))
        }
    }

    fn is_valid(&self, index: u32) -> Result<(), PersitenceSetError> {
        unsafe { self.engine_server.IsClientConnected(index) }
            .then_some(())
            .or_else(|| self.ignore_non_connected.then_some(()))
            .ok_or(PersitenceSetError::NotConnected)
            .and_then(|_| {
                unsafe { self.engine_server.IsPersitentDataAvailable(index) }
                    .then_some(())
                    .ok_or(PersitenceSetError::InvalidPersistence)
            })
    }
}

impl std::error::Error for PersitenceSetError {}

impl Display for PersitenceSetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PersitenceSetError::NotConnected => f.write_str("NotConnected"),
            PersitenceSetError::InvalidPersistence => f.write_str("InvalidPersistence"),
            PersitenceSetError::WrongType(ty) => f.write_fmt(format_args!("WrongType({ty})")),
            PersitenceSetError::MissingPersistenceName => f.write_str("MissingPersistenceName"),
            PersitenceSetError::InvalidCString => f.write_str("InvalidCString"),
        }
    }
}

impl ClientIndex for &CClient {
    fn get_index(&self) -> u32 {
        self.m_nHandle as u32
    }
}

impl ClientIndex for &CPlayer {
    fn get_index(&self) -> u32 {
        get_player_index(self) as u32
    }
}
