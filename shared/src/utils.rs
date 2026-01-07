use rrplug::{
    bindings::{
        class_types::{cbaseentity::CBaseEntity, client::SignonState, cplayer::CPlayer},
        squirrelfunctions::SQUIRREL_SERVER_FUNCS,
    },
    mid::utils::try_cstring,
    prelude::{log, HSquirrelVM, Vector3},
};
use std::{
    ffi::{c_char, c_void, CStr},
    marker::PhantomData,
    mem::MaybeUninit,
};

use windows_sys::Win32::System::{
    Diagnostics::Debug::WriteProcessMemory, Threading::GetCurrentProcess,
};

use crate::bindings::{
    CGameTrace, CTraceFilterSimple, Contents, EngineFunctions, Ray, ServerFunctions,
    TraceCollisionGroup, VectorAligned, ENGINE_FUNCTIONS,
};

pub struct ClassNameIter<'a> {
    // class_name: &'a CStr,
    magic_class_name: *const i8,
    server_funcs: &'a ServerFunctions,
    ent: *mut CBaseEntity,
}

impl<'a> ClassNameIter<'a> {
    pub fn new(class_name: &'a CStr, server_funcs: &'a ServerFunctions) -> Self {
        let mut magic = std::ptr::null();

        unsafe {
            (server_funcs.some_magic_function_for_class_name)(&mut magic, class_name.as_ptr())
        };

        ClassNameIter {
            server_funcs,
            ent: std::ptr::null_mut(),
            magic_class_name: magic,
        }
    }
}

impl Iterator for ClassNameIter<'_> {
    type Item = *mut CBaseEntity;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            (self.server_funcs.find_next_entity_by_class_name)(
                self.server_funcs.ent_list.cast(),
                self.ent,
                self.magic_class_name,
            )
            .as_mut()
            .inspect(|ent| self.ent = std::ptr::from_ref(*ent).cast_mut())
            .map(std::ptr::from_mut)
        }
    }
}

pub struct Pointer<'a, T> {
    pub ptr: *const T,
    marker: PhantomData<&'a T>,
}

impl<T> From<*const T> for Pointer<'_, T> {
    fn from(value: *const T) -> Self {
        Self {
            ptr: value,
            marker: PhantomData,
        }
    }
}

impl<T> From<*mut T> for Pointer<'_, T> {
    fn from(value: *mut T) -> Self {
        Self {
            ptr: value.cast_const(),
            marker: PhantomData,
        }
    }
}

impl<'a, T> From<Pointer<'a, T>> for *const T {
    fn from(val: Pointer<'a, T>) -> Self {
        val.ptr
    }
}

impl<'a, T> From<Pointer<'a, T>> for *mut T {
    fn from(val: Pointer<'a, T>) -> Self {
        val.ptr.cast_mut()
    }
}

#[inline]
pub unsafe fn iterate_c_array_sized<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &T> {
    let ptr: *const T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_ref())
}

#[inline]
pub unsafe fn iterate_c_array_sized_mut<T, const U: usize>(
    ptr: Pointer<T>,
) -> impl Iterator<Item = &mut T> {
    let ptr: *mut T = ptr.into();
    (0..U).filter_map(move |i| ptr.add(i).as_mut())
}

#[inline]
pub unsafe fn set_c_char_array<const U: usize>(buf: &mut [c_char; U], new: &str) {
    *buf = [0; U]; // null everything
    buf.iter_mut()
        .zip(new.as_bytes())
        .for_each(|(buf_char, new)| *buf_char = *new as i8);
    buf[U - 1] = 0; // also null last byte
}

#[inline]
pub fn get_c_char_array_lossy<const U: usize>(buf: &[c_char; U]) -> String {
    let index = buf
        .iter()
        .position(|c| *c == b'\0' as i8)
        .unwrap_or(buf.len());
    String::from_utf8_lossy(&buf.map(|i| i as u8)[0..index]).to_string()
}

#[inline]
pub fn get_c_char_array<const U: usize>(buf: &[i8; U]) -> Option<&str> {
    let index = buf
        .iter()
        .position(|c| *c == b'\0' as i8)
        .unwrap_or(buf.len());
    // SAFETY: an i8 is a valid u8
    str::from_utf8(&(unsafe { std::mem::transmute::<&[i8; U], &[u8; U]>(buf) })[0..index]).ok()
}

#[inline]
pub unsafe fn from_c_string<T: From<String>>(ptr: *const c_char) -> T {
    CStr::from_ptr(ptr).to_string_lossy().to_string().into()
}

#[allow(unused)]
#[inline]
pub unsafe fn patch(addr: usize, bytes: &[u8]) {
    WriteProcessMemory(
        GetCurrentProcess(),
        addr as *const c_void,
        bytes as *const _ as *const c_void,
        bytes.len(),
        std::ptr::null_mut(),
    );
}

pub fn send_client_print(player: &CPlayer, msg: &str) -> Option<()> {
    let engine = ENGINE_FUNCTIONS.wait();

    let client = unsafe { engine.client_array.add(get_player_index(player)).as_ref()? };
    if !client.m_bFullyAuthenticated || client.m_nSignonState != SignonState::CONNECTED {
        return None;
    }

    let msg = try_cstring(msg).ok()?;

    unsafe { (engine.cgame_client_printf)(client, msg.as_ptr()) };

    None
}

#[doc(alias = "get_from_ehandle")]
pub fn lookup_ent(handle: i32, server_funcs: &ServerFunctions) -> Option<&CBaseEntity> {
    let entry_index = (handle & 0xffff) as usize;
    let serial_number = handle >> 0x10;

    if handle == -1
        || entry_index > 0x3fff
        || unsafe {
            server_funcs
                .ent_list
                .add(entry_index)
                .as_ref()?
                .serial_number
        } != serial_number
    {
        return None;
    }

    unsafe {
        server_funcs
            .ent_list
            .add(entry_index)
            .as_ref()?
            .ent
            .as_ref()
    }
}

/// who really knows what it does lol
pub fn get_entity_handle(player: &CBaseEntity) -> i32 {
    player.m_RefEHandle
}

pub fn get_net_var(
    player: &CPlayer,
    netvar: &CStr,
    index: i32,
    server_funcs: &ServerFunctions,
) -> Option<i32> {
    let mut buf = [0; 4];
    lookup_ent(player.m_playerScriptNetDataGlobal, server_funcs)
        .map(|ent| unsafe {
            (server_funcs.get_net_var_from_ent)(ent, netvar.as_ptr(), index, buf.as_mut_ptr())
        })
        .map(|_| buf[0])
}

pub fn get_ents_by_class_name<'a>(
    name: &'a CStr,
    server_funcs: &'a ServerFunctions,
) -> impl Iterator<Item = *mut CBaseEntity> + 'a {
    ClassNameIter::new(name, server_funcs)
}

pub fn get_weaponx_name<'a>(
    weapon: &'a CBaseEntity,
    server_funcs: &ServerFunctions,
) -> Option<&'a str> {
    unsafe {
        rrplug::mid::utils::str_from_char_ptr(
            server_funcs
                .weapon_names_string_table
                .as_ref()?
                .as_ref()?
                .GetString(
                    *std::ptr::from_ref(weapon).cast::<i32>().byte_offset(0x12d8), // this is the name index TODO: make this a actual field in some struct
                ),
        )
    }
}

pub const fn nudge_type<O>(input: O) -> O {
    input
}

pub fn get_player_index(player: &CPlayer) -> usize {
    (player.m_Network.m_edict - 1) as usize
}

pub fn trace_ray(
    start: Vector3,
    end: Vector3,
    ent: Option<&CBaseEntity>,
    collision_group: TraceCollisionGroup,
    contents: Contents,
    server_funcs: &ServerFunctions,
    engine_funcs: &EngineFunctions,
) -> CGameTrace {
    let ray = Ray {
        start: VectorAligned { vec: start, w: 0. },
        delta: VectorAligned {
            vec: end - start,
            w: 0.,
        },
        offset: VectorAligned {
            vec: Vector3::ZERO,
            w: 0.,
        },
        unk3: 0.,
        unk4: 0,
        unk5: 0.,
        unk6: 1103806595072,
        unk7: 0.,
        is_ray: true,
        is_swept: false,
        is_smth: false,
        flags: 0,
        unk8: 0,
    };
    let mut result = MaybeUninit::zeroed();

    let filter: *const CTraceFilterSimple = &CTraceFilterSimple {
        vtable: server_funcs.simple_filter_vtable,
        unk: 0,
        pass_ent: ent
            .map(|e| e as *const CBaseEntity)
            .unwrap_or(std::ptr::null()),
        should_hit_func: std::ptr::null(),
        collision_group: collision_group as i32,
    };

    unsafe {
        (engine_funcs.trace_ray_filter)(
            // what why is there a deref here?
            *(server_funcs.ctraceengine) as *const c_void,
            &ray,
            contents.bits(),
            filter.cast(),
            result.as_mut_ptr(),
        );
    }

    unsafe { result.assume_init() }
}

#[allow(clippy::too_many_arguments)]
pub fn trace_hull(
    start: Vector3,
    end: Vector3,
    min: Vector3,
    max: Vector3,
    ent: Option<&CBaseEntity>,
    collision_group: TraceCollisionGroup,
    contents: Contents,
    server_funcs: &ServerFunctions,
    engine_funcs: &EngineFunctions,
) -> CGameTrace {
    let mut ray = unsafe {
        let mut ray = MaybeUninit::zeroed();
        (server_funcs.create_trace_hull)(ray.as_mut_ptr(), &start, &end, &min, &max);
        ray.assume_init()
    };

    ray.is_smth = false;

    let mut result = MaybeUninit::zeroed();

    let filter: *const CTraceFilterSimple = &CTraceFilterSimple {
        vtable: server_funcs.simple_filter_vtable,
        unk: 0,
        pass_ent: ent
            .map(|e| e as *const CBaseEntity)
            .unwrap_or(std::ptr::null()),
        should_hit_func: std::ptr::null(),
        collision_group: collision_group as i32,
    };

    unsafe {
        (engine_funcs.trace_ray_filter)(
            // what why is there a deref here?
            *(server_funcs.ctraceengine) as *const c_void,
            &ray,
            contents.bits(),
            filter.cast(),
            result.as_mut_ptr(),
        );
    }

    unsafe { result.assume_init() }
}

pub fn is_alive(ent: &CBaseEntity) -> bool {
    ent.m_lifeState == 0
}

pub fn get_value_for_key_string(ent: &CBaseEntity, key: &CStr) -> Option<String> {
    // let mut buf = [0i8; 1028];

    unsafe {
        (0..ent.genericKeyValueCount)
            .filter_map(|i| {
                let keyvalue = ent
                    .genericKeyValues
                    .byte_offset(i as isize * 0x10)
                    .cast::<*const c_char>();
                Some((
                    CStr::from_ptr(keyvalue.as_ref()?.as_ref()?),
                    CStr::from_ptr(keyvalue.byte_offset(0x8).as_ref()?.as_ref()?),
                ))
            })
            .find_map(|(key_cmp, value)| {
                (key_cmp == key).then(|| value.to_string_lossy().to_string())
            })
    }
}

pub fn get_global_net_int(global: impl AsRef<str>, server_funcs: &ServerFunctions) -> i32 {
    log::info!("global net int");
    let compiler_keywords = unsafe {
        let sqvm = (server_funcs.sq_getcompilerkeywords)(
            (server_funcs.get_some_net_var_csqvm)()
                .as_ref()
                .unwrap()
                .sqvm,
        )
        .cast::<HSquirrelVM>()
        .as_mut()
        .unwrap();
        (SQUIRREL_SERVER_FUNCS.wait().sq_getstring)(sqvm, 2)
    };

    #[allow(clippy::precedence)]
    unsafe {
        log::info!(
            "global net index {}",
            CStr::from_ptr(compiler_keywords).to_string_lossy()
        );
        let index = (server_funcs.get_net_var_index)(
            compiler_keywords,
            compiler_keywords,
            0,
            0xffffffff,
            std::ptr::null_mut(),
        );

        log::info!("global net int {}", global.as_ref());
        let mut buf = 0i32;
        if let Some(ent) = (*server_funcs.net_var_global_ent).as_ref() {
            (server_funcs.get_net_var_from_ent)(
                ent,
                try_cstring(global.as_ref()).unwrap().as_ptr(),
                index,
                &mut buf,
            );
        }

        buf
    }
}

pub fn get_global_net_float(global: impl AsRef<str>, server_funcs: &ServerFunctions) -> f32 {
    // unsafe {
    //     (server_funcs.get_global_net_float)(
    //         try_cstring(global.as_ref()).unwrap_or_default().as_ptr(),
    //     ) as f32
    // }
    0.
}
