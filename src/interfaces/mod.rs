#![allow(clippy::too_many_arguments)]

use std::cell::RefCell;

use libc::{c_char, c_void};
use once_cell::sync::OnceCell;
use rrplug::{
    create_external_interface, exports::windows::Win32::Foundation::HMODULE, high::vector::QAngle,
    prelude::*,
};

use crate::{bindings::ENGINE_FUNCTIONS, utils::create_source_interface};

use self::concommands::register_concommands;

mod concommands;
mod hooks;

pub static ENGINE_INTERFACES: OnceCell<EngineInterfaces> = OnceCell::new();
pub static HAS_BEEN_FORCED_BOX: EngineGlobal<RefCell<Option<ConVarStruct>>> =
    EngineGlobal::new(RefCell::new(None));

pub struct EngineInterfaces {
    pub debug_overlay: &'static IVDebugOverlay, // since it's a ptr to class which has a ptr to vtable
    pub engine_server: *mut *const [*const c_void; 211],
    pub engine_client: *mut *const [*const c_void; 200],
}

unsafe impl Sync for EngineInterfaces {}
unsafe impl Send for EngineInterfaces {}

#[derive(Debug)]
pub struct Interfaces;

impl Plugin for Interfaces {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        "Interfaces",
        "Interfaces",
        "Interfaces",
        PluginContext::all(),
    );

    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_sqvm_created(&self, _sqvm_handle: &CSquirrelVMHandle, _engine_token: EngineToken) {}

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        hooks::hook(dll_ptr);

        if let WhichDll::Server = dll_ptr.which_dll() {
            let convar = ConVarStruct::find_convar_by_name("enable_debug_overlays", token)
                .expect("enable_debug_overlays should exist");
            convar.set_value_i32(1, token);
        }

        let Some(engine) = engine else { return };

        engine
            .register_convar("try_set_pos", "1", "test", 0, token)
            .unwrap();

        let box_convar = ConVarStruct::try_new(
            &ConVarRegister::new(
                "force_mp_box",
                "1",
                0,
                "will put into mp_box if you are not on mp_box",
            ),
            token,
        )
        .unwrap();

        _ = HAS_BEEN_FORCED_BOX.get(token).replace(Some(box_convar));

        register_concommands(engine, token);

        _ = unsafe {
            ENGINE_INTERFACES.set(EngineInterfaces {
                debug_overlay: IVDebugOverlay::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VDebugOverlay004",
                )
                .unwrap(),
                engine_server: create_source_interface::<*const [*const c_void; 211]>(
                    "engine.dll\0".as_ptr().cast(),
                    "VEngineServer022\0".as_ptr().cast(),
                )
                .unwrap(),
                engine_client: create_source_interface::<*const [*const c_void; 200]>(
                    ("engine.dll\0").as_ptr().cast(),
                    ("VEngineClient013\0").as_ptr().cast(),
                )
                .unwrap(),
            })
        };
    }

    fn runframe(&self, token: EngineToken) {
        match HAS_BEEN_FORCED_BOX.get(token).borrow().as_ref() {
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
                            "map mp_box\0".as_ptr().cast(),
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

        if ConVarStruct::find_convar_by_name("try_set_pos", token)
            .map(|convar| convar.get_value_i32() == 1)
            .unwrap_or_default()
        {};

        let Ok(convar) = ConVarStruct::find_convar_by_name("idcolor_ally", token) else {
            return;
        };

        let Ok(line) = convar.get_value_str() else {
            return;
        };

        let Some(color) = line.split(' ').next() else {
            return;
        };

        let Ok(value) = color.parse::<f32>() else {
            return;
        };

        convar.set_value_string(
            format!(
                "{:.*} 0.100 1.000 8",
                3,
                if value < 1. { value + 0.01 } else { 0. }
            ),
            token,
        )
    }
}

create_external_interface! {
    pub IVDebugOverlay + IVDebugOverlayMod => {
        pub fn AddEntityTextOverlay(iEntIndex: i32, iLineOffset: i32, fDuration: f32, r: i32, g: i32, b: i32, a: i32, fmt: *const c_char) -> (); // was variadic

        pub fn AddBoxOverlay(origin: *const Vector3, mins: *const Vector3, max: *const Vector3, orientation: *const QAngle, r: i32, g: i32, b: i32, a: i32, doDepthTest: bool, duration: f32) -> ();
        pub fn AddSphereOverlay(vOrigin: *const Vector3, flRadius: f32, nTheta: i32, nPhi: i32, r: i32, g: i32, b: i32, a: i32, flDuration: f32) -> ();
        pub fn AddTriangleOverlay(p1: *const Vector3, p2: *const Vector3,p3: *const Vector3, r: i32, g: i32, b: i32, a: i32, doDepthTesto: i32, duration: i32) -> ();
        pub fn AddLineOverlay(origin: *const Vector3, dest: *const Vector3, r: i32, g: i32, b: i32, doDepthTest: bool, duration: f32) -> ();
        pub(self) fn sub_1800AA120(a2: i32, a3: i32, a4: i32, a5: i32, a6: i32, a7: c_char, a8: i32) -> ();

        pub fn AddTextOverlay(a2: i64, a3: i64, a4: i64,a5: *const c_char) -> (); // was variadic
        pub fn AddTextOverlay2(a2: i64, a3: i64, a4: *const c_char) -> (); // was variadic

        pub(self) fn sub_1800AA1B0() -> ();
        pub(self) fn sub_1800AA210() -> ();

        pub fn AddSweptBoxOverlay(start: *const Vector3, end: *const Vector3, mins: *const Vector3, max: *const Vector3, angles: *const QAngle, r: i32, g: i32, b: i32, a: i32, flDuration: f32) -> ();
        pub fn AddGridOverlay(vPos: *const Vector3) -> ();
        pub fn AddCoordFrameOverlay(frame: *const () , flScale: f32, vColorTable: *const [i32;3]) -> (); // Untested

        pub(self) fn sub_1800AC1B0() -> ();
        pub(self) fn sub_1800AC280() -> ();
        pub(self) fn sub_1800ADE20() -> ();
        pub(self) fn sub_1800AAB90() -> ();
        pub(self) fn sub_1800AAA60() -> ();
        pub(self) fn sub_1800AB670() -> ();
        pub(self) fn sub_1800ADEC0() -> ();
        pub(self) fn sub_1800ABDD0() -> ();
        pub(self) fn sub_1800ADE80() -> ();

        pub fn AddTextOverlayRGB(origin: *const Vector3, line_offset : i32, duration: f32, r: f32, g: f32, b: f32, alpha: f32, format: *const c_char) -> (); // was variadic
        pub fn AddTextOverlayRGBInt(origin: *const Vector3, line_offset: i32, duration: f32, r: i32, g: i32, b: i32, a: i32, format: *const c_char) -> (); // was variadic

        // pub(self) fn sub_1800A9F00(void* a2, void* a3, int a4, int a5, int a6, int a7, char a8) -> ();
        // pub(self) fn sub_1800A9870(void* a2, void* a3, void* a4, void* a5, void* a6, void* a7) -> ();
        pub(self) fn sub_1800A9F00() -> ();
        pub(self) fn sub_1800A9870() -> ();

        pub(self) fn sub_1800AD520() -> ();
        pub(self) fn sub_1800AC180() -> ();
        pub(self) fn sub_1800ADF70() -> ();
        pub(self) fn sub_1800AC260() -> ();
        pub(self) fn sub_1800ACC00() -> ();
    }

}

// class IVDebugOverlay
// {
//   public:
// };
// };
