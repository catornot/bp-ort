use libloading::{
    library_filename,
    os::windows::{Library, LOAD_WITH_ALTERED_SEARCH_PATH},
};
use once_cell::sync::Lazy;
use std::{
    ffi::{c_char, c_void},
    mem,
    path::PathBuf,
};

use crate::{
    bots_detour::hook_server,
    structs::{
        cbaseclient::CbaseClientPtr,
        clientarray::{ClientArray, ClientArrayPtr},
    },
};

static EXE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    std::env::current_exe()
        .expect("Unable to get the path to the executable")
        .parent()
        .expect("Executable path has no parent dir")
        .to_path_buf()
});

pub type PServer = *const c_void;
pub type BotName =  *const c_char;
pub type ServerGameClients = *const c_void;
pub type CbasePlayer = *mut c_void;
pub type PlayerByIndex = unsafe extern "C" fn(i32) -> CbasePlayer;
pub type ClientFullyConnected = unsafe extern "C" fn(ServerGameClients, u16, bool);
pub type RunNullCommand = unsafe extern "C" fn(CbasePlayer);
pub type CreateFakeClient = unsafe extern "C" fn(
    PServer,
    BotName,
    *const c_char,
    *const c_char,
    i32,
) -> CbaseClientPtr;

#[derive(Debug)]
pub struct SourceEngineData {
    pub server: PServer,
    pub game_clients: ServerGameClients,
    pub create_fake_client: CreateFakeClient,
    pub client_fully_connected: ClientFullyConnected,
    pub run_null_command: RunNullCommand,
    pub client_array: ClientArray,
    pub player_by_index: PlayerByIndex,
}

unsafe impl Send for SourceEngineData {}

impl SourceEngineData {
    pub fn load_server(&mut self) {
        let path = EXE_DIR.clone().join(library_filename("server"));

        log::info!("loading server.dll from path {}", path.display());

        let handle_server = match unsafe { Library::load_with_flags(path, 0) } {
            Ok(lib) => lib.into_raw() as usize,
            Err(err) => {
                log::error!("{err}");
                return;
            }
        };

        self.client_fully_connected = unsafe { mem::transmute(handle_server + 0x153B70) };
        self.run_null_command = unsafe { mem::transmute(handle_server + 0x5A9FD0) };
        self.player_by_index = unsafe { mem::transmute(handle_server + 0x26AA10) };

        hook_server(handle_server);

        if let Err(err) = unsafe { Library::from_raw(handle_server as *mut _).close() } {
            log::error!("couldn't close the handle_engine; {err}")
        }
    }

    pub fn load_engine(&mut self) {
        let path = EXE_DIR
            .clone()
            .join("bin")
            .join("x64_retail")
            .join("engine.dll");

        log::info!("loading engine.dll from path {}", path.display());

        let handle_engine =
            match unsafe { Library::load_with_flags(path, LOAD_WITH_ALTERED_SEARCH_PATH) } {
                Ok(lib) => lib.into_raw() as usize,
                Err(err) => {
                    log::error!("{err}");
                    return;
                }
            };

        self.server = (handle_engine + 0x12A53D40) as PServer;
        self.game_clients = (handle_engine + 0x13F0AAA8) as ServerGameClients;
        self.create_fake_client = unsafe { mem::transmute(handle_engine + 0x114C60) };
        self.client_array = ClientArray::new((handle_engine + 0x12A53F90) as ClientArrayPtr);

        if let Err(err) = unsafe { Library::from_raw(handle_engine as *mut _).close() } {
            log::error!("couldn't close the handle_engine; {err}")
        }
    }
}
