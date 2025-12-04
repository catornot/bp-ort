use std::{
    mem,
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use retour::static_detour;
use rrplug::{bindings::cvar::convar::ConVar, prelude::*};
use shared::bindings::{
    CLIENT_FUNCTIONS, ClientFunctions, ENGINE_FUNCTIONS, EngineFunctions, MATSYS_FUNCTIONS,
    MatSysFunctions, SERVER_FUNCTIONS, ServerFunctions,
};
use windows_sys::Win32::Networking::WinSock::{
    AF_INET6, IN6_ADDR, IN6_ADDR_0, IPPROTO_TCP, SOCKADDR, SOCKADDR_IN6, SOCKADDR_IN6_0,
    SOCKET_ERROR, WSAGetLastError, getsockname, sendto,
};

static_detour! {
    static CreateUdpSocket: extern "C" fn(*mut (), *mut (), i32) -> i32;
}

pub struct R2Mole {
    socket: Mutex<Vec<usize>>,
    port: OnceLock<&'static ConVar>,
    last_heartbeat: AtomicU64,
}

impl Plugin for R2Mole {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"r2mole", c"R2MOLE001", c"R2MOLE", PluginContext::all());

    fn new(_reloaded: bool) -> Self {
        Self {
            socket: Vec::new().into(),
            port: OnceLock::new(),
            last_heartbeat: AtomicU64::new(0),
        }
    }

    fn plugins_loaded(&self, _engine_token: EngineToken) {}

    fn on_dll_load(
        &self,
        _engine: Option<&EngineData>,
        dll_ptr: &DLLPointer,
        _engine_token: EngineToken,
    ) {
        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        if let WhichDll::Engine = dll_ptr.which_dll() {
            unsafe {
                CreateUdpSocket
                    .initialize(
                        mem::transmute::<
                            *const std::ffi::c_void,
                            extern "C" fn(*mut (), *mut (), i32) -> i32,
                        >(dll_ptr.get_dll_ptr().offset(0x21abc0)),
                        create_udp_socket_hook,
                    )
                    .expect("cannot not have a valid hook")
                    .enable()
                    .expect("should be able to enable this hook");
            }

            _ = self.port.set(unsafe {
                dll_ptr
                    .get_dll_ptr()
                    .byte_offset(0x13FA6070)
                    .cast::<ConVar>()
                    .as_ref()
                    .expect("how is this null lol")
            });
        }
    }

    fn runframe(&self, engine_token: EngineToken) {
        // if let Ok(masterserver_cvar) =
        //     ConVarStruct::find_convar_by_name("ns_masterserver_hostname", engine_token)
        // {
        //     masterserver_cvar.set_value_string("http://127.0.0.1:8334", engine_token);
        // }

        let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return;
        };

        if Duration::from_secs(self.last_heartbeat.load(Ordering::Relaxed))
            .saturating_add(Duration::new(5, 0))
            >= now
        {
            return;
        }
        self.last_heartbeat.store(
            now.as_secs(), // clueless
            Ordering::Relaxed,
        );

        let Some(mut socket) = ENGINE_FUNCTIONS
            .get()
            .and_then(|_| self.socket.try_lock().ok())
        else {
            return;
        };

        if socket.is_empty() {
            return;
        }

        if socket.len() > 1 {
            let port = self.port.wait().m_Value.m_nValue as u16;
            socket.retain(|socket| unsafe { filter_sockets(*socket, port) });
        };
        debug_assert!(socket.len() == 1, "should be one socket left by now");
        let sock = socket.first().copied().expect("bruh how");

        unsafe {
            let addr = SOCKADDR_IN6 {
                sin6_family: AF_INET6,
                sin6_port: 3400u16.to_be(),
                sin6_addr: IN6_ADDR {
                    u: IN6_ADDR_0 {
                        Byte: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
                    },
                },
                sin6_flowinfo: 0,
                Anonymous: SOCKADDR_IN6_0 { sin6_scope_id: 0 },
            };

            let data = b"hi 222";
            if sendto(
                sock,
                data.as_ptr(),
                data.len() as i32,
                0,
                (std::ptr::from_ref(&addr)).cast(),
                std::mem::size_of::<SOCKADDR_IN6>() as i32,
            ) == SOCKET_ERROR
            {
                log::error!("sendto failed: {}", WSAGetLastError());
            }
        };
    }
}

entry!(R2Mole);

unsafe fn filter_sockets(socket: usize, port: u16) -> bool {
    let mut socket_addr: mem::MaybeUninit<SOCKADDR_IN6> = mem::MaybeUninit::zeroed();
    let mut addr_len = size_of::<SOCKADDR_IN6>() as i32;
    unsafe {
        getsockname(
            socket,
            socket_addr.as_mut_ptr() as *mut SOCKADDR,
            &mut addr_len,
        ) == 0
            && u16::from_be(socket_addr.assume_init().sin6_port) == port
    }
}

fn create_udp_socket_hook(unk1: *mut (), unk2: *mut (), protocol: i32) -> i32 {
    let socket = CreateUdpSocket.call(unk1, unk2, protocol);

    // no need to capture tcp ports
    if protocol != IPPROTO_TCP {
        let mut guard = PLUGIN.wait().socket.lock().unwrap();
        guard.push(socket as usize);
    }

    socket
}
