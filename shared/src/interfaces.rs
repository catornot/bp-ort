use rrplug::{create_external_interface, prelude::*};
use std::ffi::{c_char, c_void};

create_external_interface! {
    pub IVDebugOverlay + IVDebugOverlayMod => {
        pub fn AddEntityTextOverlay(iEntIndex: i32, iLineOffset: i32, fDuration: f32, r: i32, g: i32, b: i32, a: i32, fmt: *const c_char) -> (); // was variadic

        pub fn AddBoxOverlay(origin: *const Vector3, mins: *const Vector3, max: *const Vector3, orientation: *const Vector3, r: i32, g: i32, b: i32, a: i32, doDepthTest: bool, duration: f32) -> ();
        pub fn AddSphereOverlay(vOrigin: *const Vector3, flRadius: f32, nTheta: i32, nPhi: i32, r: i32, g: i32, b: i32, a: i32, flDuration: f32) -> ();
        pub fn AddTriangleOverlay(p1: *const Vector3, p2: *const Vector3,p3: *const Vector3, r: i32, g: i32, b: i32, a: i32, doDepthTesto: i32, duration: i32) -> ();
        pub fn AddLineOverlay(origin: *const Vector3, dest: *const Vector3, r: i32, g: i32, b: i32, doDepthTest: bool, duration: f32) -> ();
        pub(self) fn sub_1800AA120(a2: i32, a3: i32, a4: i32, a5: i32, a6: i32, a7: c_char, a8: i32) -> ();

        pub fn AddTextOverlay(a2: i64, a3: i64, a4: i64,a5: *const c_char) -> (); // was variadic
        pub fn AddTextOverlay2(a2: i64, a3: i64, a4: *const c_char) -> (); // was variadic

        pub(self) fn sub_1800AA1B0() -> ();
        pub(self) fn sub_1800AA210() -> ();

        pub fn AddSweptBoxOverlay(start: *const Vector3, end: *const Vector3, mins: *const Vector3, max: *const Vector3, angles: *const Vector3, r: i32, g: i32, b: i32, a: i32, flDuration: f32) -> ();
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
        pub fn Changelevel(s1: *const c_char, s2: *const c_char) -> ();
        pub(self) fn sub_18011B140() -> ();
        pub(self) fn sub_18011B410() -> ();
        pub(self) fn sub_18011B6F0() -> ();
        pub(self) fn sub_18011B3A0() -> ();
        pub(self) fn sub_18011B3C0() -> ();
        pub fn GetLaunchOptions() -> *const c_void;

        pub fn PrecacheModel(name: *const c_char) -> i32;
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

        pub fn FadeClientVolume(pEdict: *const Edict, flFadePercent: f32, flFadeOutSeconds: f32, flHoldTime: f32, flFadeInSeconds: f32) -> ();

        pub fn ServerCommand(szCommand: *const c_char) -> ();
        pub fn ServerExecute() -> ();

        pub fn ClientCommand(pEdict: *const Edict, szFmt: *const c_char) -> (); // was varidic

        pub fn LightStyle(nStyle: i32, szVal: *const c_char) -> ();

        pub fn UserMessageBegin(a2: i64, a3: i32, a4: i64, a5: i32) -> *const c_void;
        pub fn UserMessageEnd() -> ();

        pub fn ClientPrintf(nEdict: Edict, szMsg: *const c_char) -> ();

        pub fn Con_NPrintf(nPos: i32, szFmt: *const c_char) -> ();// was varidic
        pub fn Con_NXPrintf(pInfo: *const c_void, szFmt: *const c_char)-> ();// was varidic

        pub(self) fn sub_18011CDD0() -> ();
        pub(self) fn sub_18011B190() -> ();
        pub(self) fn sub_18011CD10() -> ();
        pub(self) fn sub_18011CCD0() -> ();

        pub fn CrosshairAngle(nClient: Edict, flPitch: f32, flYaw: f32) -> ();

        pub fn GrantClientSidePickup(nClient: Edict, a3: i32, a4: i32, a5: *const i32, a6: i32, a7: i32) -> bool;

        pub fn GetGameDir(szGetGameDir: *mut c_char, nMaxlength: i32) -> ();

        pub fn CompareFileTime(szFilename1: *const c_char, szFilename2: *const c_char, iCompare: *const i32) -> i32;

        pub fn LockNetworkStringTables(bLock: bool) -> ();

        pub(self) fn sub_18011AD70() -> ();
        pub(self) fn sub_18011AD30() -> ();
        pub(self) fn sub_18011AD80() -> ();

        pub fn CreateFakeClient(szName: *const c_char, szUnk: *const c_char, szPlaylist: *const c_char, nTeam: i32) -> Edict;

        // started from 0
        pub fn fn_44() -> ();
        pub fn fn_45() -> ();
        pub fn fn_46() -> ();
        pub fn fn_47() -> ();
        pub fn fn_48() -> ();
        pub fn fn_49() -> ();
        pub fn fn_50() -> ();
        pub fn fn_51() -> ();
        pub fn fn_52() -> ();
        pub fn fn_53() -> ();
        pub fn fn_54() -> ();
        pub fn fn_55() -> ();
        pub fn fn_56() -> ();
        pub fn fn_57() -> ();
        pub fn fn_58() -> ();
        pub fn fn_59() -> ();
        pub fn fn_60() -> ();
        pub fn fn_61() -> ();
        pub fn fn_62() -> ();
        pub fn fn_63() -> ();
        pub fn fn_64() -> ();
        pub fn fn_65() -> ();
        pub fn fn_66() -> ();
        pub fn fn_67() -> ();
        pub fn fn_68() -> ();
        pub fn fn_69() -> ();
        pub fn fn_70() -> ();
        pub fn fn_71() -> ();
        pub fn fn_72() -> ();
        pub fn fn_73() -> ();
        pub fn fn_74() -> ();
        pub fn fn_75() -> ();
        pub fn fn_76() -> ();
        pub fn fn_77() -> ();
        pub fn fn_78() -> ();
        pub fn fn_79() -> ();
        pub fn fn_80() -> ();
        pub fn fn_81() -> ();
        pub fn fn_82() -> ();
        pub fn fn_83() -> ();
        pub fn fn_84() -> ();
        pub fn fn_85() -> ();
        pub fn fn_86() -> ();
        pub fn fn_87() -> ();
        pub fn fn_88() -> ();
        pub fn fn_89() -> ();
        pub fn fn_90() -> ();
        pub fn fn_91() -> ();
        pub fn fn_92() -> ();
        pub fn fn_93() -> ();
        pub fn fn_94() -> ();
        pub fn fn_95() -> ();
        pub fn fn_96() -> ();
        pub fn fn_97() -> ();
        pub fn fn_98() -> ();
        pub fn fn_99() -> ();
        pub fn fn_100() -> ();
        pub fn fn_101() -> ();
        pub fn fn_102() -> ();
        pub fn fn_103() -> ();
        pub fn fn_104() -> ();
        pub fn fn_105() -> ();
        pub fn fn_106() -> ();
        pub fn fn_107() -> ();
        pub fn fn_108() -> ();
        pub fn fn_109() -> ();
        pub fn fn_110() -> ();
        pub fn fn_111() -> ();
        pub fn fn_112() -> ();
        pub fn fn_113() -> ();
        pub fn fn_114() -> ();
        pub fn fn_115() -> ();
        pub fn fn_116() -> ();
        pub fn fn_117() -> ();
        pub fn fn_118() -> ();
        pub fn fn_119() -> ();
        pub fn fn_120() -> ();
        pub fn fn_121() -> ();
        pub fn fn_122() -> ();
        pub fn fn_123() -> ();
        pub fn fn_124() -> ();
        pub fn fn_125() -> ();
        pub fn fn_126() -> ();
        pub fn fn_127() -> ();
        pub fn fn_128() -> ();
        pub fn fn_129() -> ();
        pub fn fn_130() -> ();
        pub fn fn_131() -> ();
        pub fn fn_132() -> ();
        pub fn fn_133() -> ();
        pub fn fn_134() -> ();
        pub fn fn_135() -> ();
        pub fn fn_136() -> ();
        pub fn fn_137() -> ();
        pub fn fn_138() -> ();
        pub fn fn_139() -> ();
        pub fn fn_140() -> ();
        pub fn fn_141() -> ();
        pub fn fn_142() -> ();
        pub fn fn_143() -> ();
        pub fn fn_144() -> ();
        pub fn fn_145() -> ();
        pub fn fn_146() -> ();
        pub fn fn_147() -> ();
        pub fn fn_148() -> ();
        pub fn fn_149() -> ();
        pub fn fn_150() -> ();
        pub fn fn_151() -> ();
        pub fn fn_152() -> ();
        pub fn fn_153() -> ();
        pub fn fn_154() -> ();
        pub fn fn_155() -> ();
        pub fn fn_156() -> ();
        pub fn fn_157() -> ();
        pub fn fn_158() -> ();
        pub fn fn_159() -> ();
        pub fn fn_160() -> ();
        pub fn fn_161() -> ();
        pub fn fn_162() -> ();
        pub fn fn_163() -> ();
        pub fn fn_164() -> ();
        pub fn fn_165() -> ();
        pub fn fn_166() -> ();
        pub fn fn_167() -> ();
        pub fn fn_168() -> ();
        pub fn fn_169() -> ();
        pub fn fn_170() -> ();
        pub fn fn_171() -> ();
        pub fn fn_172() -> ();
        pub fn fn_173() -> ();
        pub fn fn_174() -> ();
        pub fn fn_175() -> ();
        pub fn fn_176() -> ();
        pub fn fn_177() -> ();
        pub fn fn_178() -> ();
        pub fn fn_179() -> ();
        pub fn fn_180() -> ();
        pub fn fn_181() -> ();
        pub fn fn_182() -> ();
        pub fn fn_183() -> ();
        pub fn fn_184() -> ();

        pub fn IsClientConnected(clientIndex: u32) -> bool;
        pub fn IsPersitentDataAvailable(clientIndex: u32) -> bool;
        pub fn GetPersistenceDataType(clientIndex: u32, persistenceName: *const c_char, out: *mut u64) -> u32;
        pub fn PersitenceUnk1(clientIndex: u32) -> ();
        pub fn PersitenceUnk2(clientIndex: u32) -> ();
        pub fn PersitenceUnk3(clientIndex: u32) -> ();
        pub fn PersitenceUnk4(clientIndex: u32) -> ();
        pub fn PersitenceUnk5(clientIndex: u32) -> ();
        pub fn PersitenceUnk6(clientIndex: u32) -> ();
        pub fn PersitenceUnk7(clientIndex: u32) -> ();
        pub fn PersitenceUnk8(clientIndex: u32) -> ();
        pub fn SetPersistentInt1(clientIndex: u32, persistenceType: u64, persistence: i32) -> bool;
        pub fn SetPersistentInt2(clientIndex: u32, persistenceType: u64, persistence: i32) -> bool;
        pub fn SetPersistentString(clientIndex: u32, persistenceType: u64, persistence: *const c_char) -> bool;


        // not full vtable
    }
}

create_external_interface! {
    pub CNetworkStringTable + CNetworkStringTableMod => {
      pub fn destructor() -> ();
      pub fn unk_1() -> ();
      pub fn unk_2() -> ();
      pub fn unk_3() -> ();
      pub fn GetMaxStrings() -> u32;
      pub fn GetEntryBits() -> u32;
      pub fn SetTicks(ticks: u32) -> ();
      pub fn ChangedSinceTick(ticks: u32) -> bool;
      pub fn AddString(isServer: bool, key: *const c_char, length: i32, userdata: *mut c_void) -> i32;
      pub fn GetString(stringNumber: i32) -> *const c_char;
      pub fn unk_10() -> ();
      pub fn unk_11() -> ();
      pub fn unk_12() -> ();
      pub fn unk_13() -> ();
      pub fn ReadStringTable() -> *mut ();
      pub fn unk_15() -> ();
      pub fn unk_16() -> ();
      pub fn unk_17() -> ();
    }
}
