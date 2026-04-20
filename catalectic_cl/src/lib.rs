use std::mem;

use retour::static_detour;
use rrplug::{bindings::squirreldatatypes::CSquirrelVM, prelude::*};

static_detour! {
    static IsGameActiveWindow: unsafe extern "C" fn() -> bool;
    static RsonLoadScripts: unsafe extern "C" fn(*mut CSquirrelVM) -> usize;
}

pub struct Catalectic;

impl Plugin for Catalectic {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"catalectic_cl",
        c"HEADLESS",
        c"CATALECTIC",
        PluginContext::CLIENT,
    );

    fn new(_reloaded: bool) -> Self {
        Self {}
    }

    fn on_dll_load(
        &self,
        _engine_data: Option<&EngineData>,
        dll_ptr: &DLLPointer,
        _engine_token: EngineToken,
    ) {
        let Some(engine) =
            matches!(dll_ptr.which_dll(), WhichDll::Engine).then_some(dll_ptr.get_dll_ptr())
        else {
            return;
        };

        unsafe {
            IsGameActiveWindow
                .initialize(
                    mem::transmute::<*const std::ffi::c_void, unsafe extern "C" fn() -> bool>(
                        engine.offset(0x1cdc80),
                    ),
                    is_game_window_active_hook,
                )
                .expect("failed to hook IsGameActiveWindow")
                .enable()
                .expect("failure to enable the IsGameActiveWindow hook");
            log::info!("hooked IsGameActiveWindow");

            RsonLoadScripts
                .initialize(
                    mem::transmute::<
                        *const std::ffi::c_void,
                        unsafe extern "C" fn(
                            *mut rrplug::bindings::squirreldatatypes::CSquirrelVM,
                        ) -> usize,
                    >(engine.offset(0x3c80e0)),
                    rson_load_scripts_hook,
                )
                .expect("failed to hook RsonLoadScripts ")
                .enable()
                .expect("failure to enable the RsonLoadScripts hook");
            log::info!("hooked RsonLoadScripts ");

            // disable window
            shared::utils::patch(engine.byte_offset(0x1CD146) as usize, &[0x90; 5]);
        }
    }
}

entry!(Catalectic);

fn is_game_window_active_hook() -> bool {
    true
}

// drop all ui scripts to prevent issues with menu stuff
fn rson_load_scripts_hook(sqvm: *mut CSquirrelVM) -> usize {
    unsafe {
        if (*sqvm).vmContext != ScriptContext::UI as i32 {
            RsonLoadScripts.call(sqvm)
        } else {
            0
        }
    }
}
