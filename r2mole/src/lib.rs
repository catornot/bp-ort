use rrplug::prelude::*;
use shared::bindings::{
    CLIENT_FUNCTIONS, ClientFunctions, ENGINE_FUNCTIONS, EngineFunctions, MATSYS_FUNCTIONS,
    MatSysFunctions, SERVER_FUNCTIONS, ServerFunctions,
};

pub struct R2Mole {}

impl Plugin for R2Mole {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"octbots", c" OCTBOTS ", c"OCTBOTS", PluginContext::all());

    fn new(_reloaded: bool) -> Self {
        Self {}
    }

    fn plugins_loaded(&self, _engine_token: EngineToken) {}

    fn on_dll_load(
        &self,
        engine: Option<&EngineData>,
        dll_ptr: &DLLPointer,
        engine_token: EngineToken,
    ) {
        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        let Some(engine_data) = engine else { return };

        engine_data
            .register_concommand("test_socket", test_socket, "", 0, engine_token)
            .expect("couldn't register a concommand");

        // _ = unsafe {
        //     ENGINE_INTERFACES.set(EngineInterfaces {
        //         debug_overlay: IVDebugOverlay::from_dll_ptr(
        //             HMODULE(dll_ptr.get_dll_ptr() as isize),
        //             "VDebugOverlay004",
        //         )
        //         .unwrap(),
        //         engine_server: IVEngineServer::from_dll_ptr(
        //             HMODULE(dll_ptr.get_dll_ptr() as isize),
        //             "VEngineServer022",
        //         )
        //         .unwrap(),
        //     })
        // };
    }

    // fn on_reload_request(&self) -> reloading::ReloadResponse {
    //     // has to be reloadable
    //     unsafe { reloading::ReloadResponse::allow_reload() }
    // }

    #[allow(unused_variables, unreachable_code)]
    fn runframe(&self, _engine_token: EngineToken) {}
}

entry!(R2Mole);

#[rrplug::concommand]
fn test_socket() -> Option<()> {
    use windows_sys::Win32::Networking::WinSock::*;

    let engine = ENGINE_FUNCTIONS.wait();
    unsafe {
        let server = engine.server.as_ref()?;

        let local_host = *gethostbyname(c"".as_ptr().cast()).as_ref()?;
        let local_ip = inet_ntoa(
            **local_host
                .h_addr_list
                .as_ref()?
                .cast_const()
                .cast::<*const IN_ADDR>(),
        );

        let addr = SOCKADDR_IN {
            sin_family: AF_INET,
            sin_addr: IN_ADDR {
                S_un: IN_ADDR_0 {
                    S_addr: inet_addr(local_ip),
                },
            },
            sin_port: htons(3400),
            sin_zero: [0; 8],
        };

        sendto(
            server.m_Socket as usize,
            b"hi".as_ptr(),
            2,
            0,
            (std::ptr::from_ref(&addr)).cast(),
            std::mem::size_of::<SOCKADDR_IN>() as i32,
        );
    };

    None
}
