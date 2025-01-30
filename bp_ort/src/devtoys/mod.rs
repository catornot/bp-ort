use rrplug::{
    bindings::cvar::convar::{FCVAR_CHEAT, FCVAR_CLIENTDLL},
    prelude::*,
};
use std::{cell::RefCell, convert::Infallible};

use crate::bindings::ENGINE_FUNCTIONS;

mod detour;
mod random_detour;
mod reversing_detour;

pub static DRAWWORLD_CONVAR: EngineGlobal<RefCell<Option<ConVarStruct>>> =
    EngineGlobal::new(RefCell::new(None));
pub static PAUSABLE_CONVAR: EngineGlobal<RefCell<Option<ConVarStruct>>> =
    EngineGlobal::new(RefCell::new(None));
pub static FORCE_BOX_CONVAR: EngineGlobal<RefCell<Option<ConVarStruct>>> =
    EngineGlobal::new(RefCell::new(None)); // move to dev toys
pub static ALLY_COLOR_CONVAR: EngineGlobal<RefCell<Option<ConVarStruct>>> =
    EngineGlobal::new(RefCell::new(None)); // move to dev toys
pub static FREE_CAM_CONVAR: EngineGlobal<RefCell<Option<ConVarStruct>>> =
    EngineGlobal::new(RefCell::new(None)); // move to dev toys

#[derive(Debug)]
pub struct DevToys;

impl Plugin for DevToys {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"devtoys", c"devtoys", c"devtoys", PluginContext::DEDICATED);
    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        match dll_ptr.which_dll() {
            WhichDll::Engine => {
                detour::hook_engine(dll_ptr.get_dll_ptr());
                reversing_detour::hook_engine(dll_ptr.get_dll_ptr());

                _ = FREE_CAM_CONVAR.get(token).borrow_mut().replace(
                    ConVarStruct::try_new(
                        &ConVarRegister::new(
                            "free_cam_active",
                            "0",
                            FCVAR_CLIENTDLL as i32,
                            "maybe a working free cam",
                        ),
                        token,
                    )
                    .expect("convar registration failed for free cam cvar"),
                );
            }
            WhichDll::Server => {
                detour::hook_server(dll_ptr.get_dll_ptr());
                reversing_detour::hook_server(dll_ptr.get_dll_ptr());

                let mut draw_convar = ConVarStruct::find_convar_by_name("r_drawworld", token)
                    .expect("r_drawworld should exist");
                draw_convar.remove_flags(FCVAR_CHEAT as i32, token);

                let mut drawworld = DRAWWORLD_CONVAR.get(token).borrow_mut();
                _ = drawworld.replace(draw_convar);

                _ = PAUSABLE_CONVAR.get(token).borrow_mut().replace(
                    ConVarStruct::find_convar_by_name("sv_pausable", token)
                        .expect("sv_pausable should exist"),
                );

                // if let Ok(convar) = ConVarStruct::find_convar_by_name("idcolor_ally", token) {
                //     ALLY_COLOR_CONVAR.get(token).borrow_mut().replace(convar);
                // }
                let convar = ConVarStruct::find_convar_by_name("enable_debug_overlays", token)
                    .expect("enable_debug_overlays should exist");
                convar.set_value_i32(cfg!(not(feature = "release")) as i32, token);
            }
            WhichDll::Client => {
                random_detour::hook_client(dll_ptr.get_dll_ptr());

                let mut fov_scale_convar = ConVarStruct::find_convar_by_name("cl_fovScale", token)
                    .expect("cl_fovscale should exist");
                let fov_ptr = unsafe { fov_scale_convar.get_raw_convar_ptr().as_mut() }
                    .expect("cl_fovscale should exist");

                log::info!("cl_fovscale {}", fov_ptr.m_bHasMax);
                log::info!("cl_fovscale {}", fov_ptr.m_bHasMin);
                log::info!("cl_fovscale {}", fov_ptr.m_fMaxVal);
                log::info!("cl_fovscale {}", fov_ptr.m_fMaxVal);

                fov_ptr.m_bHasMax = false;
                fov_ptr.m_bHasMin = false;
                fov_ptr.m_fMaxVal = f32::MIN;
                fov_ptr.m_fMaxVal = f32::MAX;

                fov_scale_convar.set_value_i32(20, token);
            }
            WhichDll::Other("materialsystem_dx11.dll") => {
                random_detour::hook_materialsystem(dll_ptr.get_dll_ptr());
            }

            _ => {}
        }

        let Some(engine) = engine else { return };

        let box_convar = ConVarStruct::try_new(
            &ConVarRegister::new(
                "force_mp_box",
                "0",
                0,
                "will put you into mp_box if you are not on mp_box",
            ),
            token,
        )
        .unwrap();

        _ = FORCE_BOX_CONVAR.get(token).replace(Some(box_convar));

        engine
            .register_concommand(
                "remove_max_min",
                remove_max,
                "removes the limits on interger convars",
                0,
                token,
            )
            .unwrap();
    }

    fn runframe(&self, token: EngineToken) {
        match FORCE_BOX_CONVAR.get(token).borrow().as_ref() {
            Some(convar) if convar.get_value_i32() == 1 => {
                let engine = ENGINE_FUNCTIONS.wait();
                let host_state = unsafe {
                    engine
                        .host_state
                        .as_mut()
                        .expect("host state should be valid")
                };

                let level_name = host_state
                    .level_name
                    .iter()
                    .cloned()
                    .filter(|i| *i != 0)
                    .filter_map(|i| char::from_u32(i as u32))
                    .collect::<String>();

                if level_name != "mp_box" {
                    log::info!("go to mp_box. NOW!");

                    unsafe {
                        (engine.cbuf_add_text)(
                            (engine.cbuf_get_current_player)(),
                            c"map mp_box".as_ptr().cast(),
                            crate::bindings::CmdSource::Code,
                        )
                    };
                    // host_state.next_state = HostState::NewGame;
                    // unsafe { set_c_char_array(&mut host_state.level_name, "mp_box") };
                } else {
                    convar.set_value_i32(0, token)
                }
            }
            None => {}
            Some(_) => {}
        }

        // if ALLY_COLOR_CONVAR
        //     .get(token)
        //     .borrow()
        //     .as_ref()
        //     .map(|convar| convar.get_value_bool())
        //     .unwrap_or_default()
        // {
        //     let client = CLIENT_FUNCTIONS.wait();
        //     if let Some(local_player) = unsafe { (client.get_local_c_player)().as_mut() } {}
        // }
        // todo

        let ally_convar = ALLY_COLOR_CONVAR.get(token).borrow_mut();
        let Some(ally_convar) = ally_convar.as_ref() else {
            return;
        };

        let Ok(line) = ally_convar.get_value_str() else {
            return;
        };

        let Some(color) = line.split(' ').next() else {
            return;
        };

        let Ok(value) = color.parse::<f32>() else {
            return;
        };

        ally_convar.set_value_string(
            format!(
                "{:.*} 0.100 1.000 8",
                3,
                if value < 1. { value + 0.01 } else { 0. }
            ),
            token,
        )
    }
}

#[rrplug::concommand]
fn remove_max(cmd: CCommandResult) -> Option<Infallible> {
    let convar_name = cmd.get_arg(0)?;

    let convar = unsafe {
        ConVarStruct::find_convar_by_name(convar_name, engine_token)
            .ok()?
            .get_raw_convar_ptr()
    };

    let convar = unsafe { convar.as_mut()? };
    convar.m_bHasMin = false;
    convar.m_bHasMax = false;
    convar.m_fMinVal = f32::MIN;
    convar.m_fMaxVal = f32::MAX;

    None
}
