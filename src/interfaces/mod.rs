#![allow(clippy::too_many_arguments)]

use libc::{c_char, c_void};
use once_cell::sync::OnceCell;
use rrplug::{
    create_external_interface, exports::windows::Win32::Foundation::HMODULE, high::vector::QAngle,
    prelude::*,
};

use self::concommands::register_concommands;

mod concommands;
mod hooks;

pub static ENGINE_INTERFACES: OnceCell<EngineInterfaces> = OnceCell::new();

pub struct EngineInterfaces {
    pub debug_overlay: &'static IVDebugOverlay, // since it's a ptr to class which has a ptr to vtable
    pub engine_server: &'static IVEngineServer,
}

unsafe impl Sync for EngineInterfaces {}
unsafe impl Send for EngineInterfaces {}

#[derive(Debug)]
pub struct Interfaces;

impl Plugin for Interfaces {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"Interfaces",
        c"Interfaces",
        c"Interfaces",
        PluginContext::all(),
    );

    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_sqvm_created(&self, _sqvm_handle: &CSquirrelVMHandle, _engine_token: EngineToken) {}

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        hooks::hook(dll_ptr);

        let Some(engine) = engine else { return };

        register_concommands(engine, token);

        _ = unsafe {
            ENGINE_INTERFACES.set(EngineInterfaces {
                debug_overlay: IVDebugOverlay::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VDebugOverlay004",
                )
                .unwrap(),
                engine_server: IVEngineServer::from_dll_ptr(
                    HMODULE(dll_ptr.get_dll_ptr() as isize),
                    "VEngineServer022",
                )
                .unwrap(),
            })
        };
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

type Edict = u16;

create_external_interface! {
    pub IVEngineServer + IVEngineServerMod => {
        pub(crate) fn Changelevel(s1: *const c_char, s2: *const c_char) -> ();
        pub(self) fn sub_18011B140() -> ();
        pub(self) fn sub_18011B410() -> ();
        pub(self) fn sub_18011B6F0() -> ();
        pub(self) fn sub_18011B3A0() -> ();
        pub(self) fn sub_18011B3C0() -> ();
        pub(crate) fn GetLaunchOptions() -> *const c_void;

        pub(crate) fn PrecacheModel(name: *const c_char) -> i32;
        pub(self) fn sub_18011B440() -> ();

        pub(self) fn sub_18011B520() -> ();

        pub(self) fn sub_18011ACB0() -> ();
        pub(self) fn sub_18011A9C0() -> ();
        pub(self) fn sub_18011AA00() -> ();
        pub(self) fn sub_18011A860() -> ();
        pub(self) fn sub_18011AD40() -> ();
        pub(self) fn sub_18011C730() -> ();
        pub(self) fn sub_18011C790() -> ();
        pub(self) fn sub_18011C8B0() -> ();
        pub(self) fn sub_18011A650() -> ();
        pub(self) fn sub_18011C870() -> ();

        pub(crate) fn FadeClientVolume(pEdict: *const Edict, flFadePercent: f32, flFadeOutSeconds: f32, flHoldTime: f32, flFadeInSeconds: f32) -> ();

        pub(crate) fn ServerCommand(szCommand: *const c_char) -> ();
        pub(crate) fn ServerExecute() -> ();

        pub(crate) fn ClientCommand(pEdict: *const Edict, szFmt: *const c_char) -> (); // was varidic

        pub(crate) fn LightStyle(nStyle: i32, szVal: *const c_char) -> ();

        pub(crate) fn UserMessageBegin(a2: i64, a3: i32, a4: i64, a5: i32) -> *const c_void;
        pub(crate) fn UserMessageEnd() -> ();

        pub(crate) fn ClientPrintf(nEdict: Edict, szMsg: *const c_char) -> ();

        pub(crate) fn Con_NPrintf(nPos: i32, szFmt: *const c_char) -> ();// was varidic
        pub(crate) fn Con_NXPrintf(pInfo: *const c_void, szFmt: *const c_char)-> ();// was varidic

        pub(self) fn sub_18011CDD0() -> ();
        pub(self) fn sub_18011B190() -> ();
        pub(self) fn sub_18011CD10() -> ();
        pub(self) fn sub_18011CCD0() -> ();

        pub(crate) fn CrosshairAngle(nClient: Edict, flPitch: f32, flYaw: f32) -> ();

        pub(crate) fn GrantClientSidePickup(nClient: Edict, a3: i32, a4: i32, a5: *const i32, a6: i32, a7: i32) -> bool;

        pub(crate) fn GetGameDir(szGetGameDir: *mut c_char, nMaxlength: i32) -> ();

        pub(crate) fn CompareFileTime(szFilename1: *const c_char, szFilename2: *const c_char, iCompare: *const i32) -> i32;

        pub(crate) fn LockNetworkStringTables(bLock: bool) -> ();

        pub(self) fn sub_18011AD70() -> ();
        pub(self) fn sub_18011AD30() -> ();
        pub(self) fn sub_18011AD80() -> ();

        pub(crate) fn CreateFakeClient(szName: *const c_char, szUnk: *const c_char, szPlaylist: *const c_char, nTeam: i32) -> Edict;

        // not full vtable
}
}
