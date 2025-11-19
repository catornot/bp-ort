use parking_lot::Mutex;
use retour::static_detour;
use rrplug::{
    bindings::{
        class_types::{cbaseentity::CBaseEntity, cplayer::CPlayer, cplayerdecoy::CPlayerDecoy},
        cvar::convar::FCVAR_GAMEDLL,
    },
    high::UnsafeHandle,
    prelude::*,
};
use shared::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::{get_player_index, lookup_ent, nudge_type},
};
use std::{collections::HashMap, sync::OnceLock};

use crate::PLUGIN;

static_detour! {
    static CreatePlayerDecoy: unsafe extern "C" fn(usize, *mut CPlayer, usize, usize) -> *mut CPlayerDecoy;
}

pub struct HoloPlus {
    holo_pilot_table: Mutex<HashMap<usize, Vec<UnsafeHandle<*mut CPlayerDecoy>>>>,
    enabled: OnceLock<ConVarStruct>,
}

impl Plugin for HoloPlus {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"holoplus",
        c"holoplus",
        c"holoplus",
        PluginContext::DEDICATED,
    );

    fn new(_: bool) -> Self {
        register_sq_functions(decoy_set_state);

        Self {
            holo_pilot_table: Mutex::new(HashMap::new()),
            enabled: OnceLock::new(),
        }
    }

    fn on_dll_load(&self, _engine: Option<&EngineData>, dll_ptr: &DLLPointer, _token: EngineToken) {
        if let WhichDll::Server = dll_ptr.which_dll()
            && !CreatePlayerDecoy.is_enabled()
        {
            if self
                .enabled
                .set(
                    ConVarStruct::try_new(
                        &ConVarRegister::new(
                            "holoplus_enabled",
                            "0",
                            FCVAR_GAMEDLL as i32,
                            "toggles holoplugs",
                        ),
                        _token,
                    )
                    .expect("couldn't create convar holoplus_enabled"),
                )
                .is_err()
            {
                panic!("couldn't set convar holoplus_enabled");
            }

            unsafe {
                CreatePlayerDecoy
                    .initialize(
                        std::mem::transmute::<*const _, _>(
                            dll_ptr.get_dll_ptr().byte_offset(0x1c40d0),
                        ),
                        create_player_decoy_hook,
                    )
                    .expect("couldn't initialize CreatePlayerDecoy")
                    .enable()
                    .expect("couldn't enable CreatePlayerDecoy");
            }
        }
    }

    fn on_sqvm_destroyed(&self, sqvm_handle: &CSquirrelVMHandle, _engine_token: EngineToken) {
        match sqvm_handle.get_context() {
            ScriptContext::SERVER => _ = self.holo_pilot_table.lock().drain(),
            ScriptContext::CLIENT | ScriptContext::UI => {}
        }
    }
}

pub fn runholoframe() {
    if let Some((globals, _engine, server)) = ENGINE_FUNCTIONS.get().and_then(|engine| {
        Some((
            unsafe { engine.globals.as_ref()? },
            engine,
            SERVER_FUNCTIONS.get()?,
        ))
    }) {
        if globals.frameCount / 2 % 4 != 0 {
            return;
        }

        for player in (0..globals.maxPlayers)
            .filter_map(|i| unsafe { (server.get_player_by_index)(i).as_ref() })
        {
            let mut holo_pilot_table = PLUGIN.wait().holoplus.holo_pilot_table.lock();
            let decoys = holo_pilot_table
                .entry(get_player_index(player))
                .or_default();

            let enabled = PLUGIN.wait().holoplus.enabled.wait().get_value_bool();
            decoys.retain(|decoy| unsafe {
                decoy
                    .copy()
                    .as_ref()
                    .map(|decoy| {
                        (server.is_alive)(nudge_type::<&CBaseEntity>(decoy)) == 1
                            && decoy.m_deathTime > globals.curTime
                            && enabled
                    })
                    .unwrap_or_default()
            });

            unsafe {
                (server.calc_absolute_velocity)(player, &std::ptr::from_ref(player), 0, 0);
                (server.calc_origin)(player, &std::ptr::from_ref(player), 0, 0);
            };

            let vel = player.m_vecAbsVelocity;
            for decoy in decoys
                .iter_mut()
                .filter_map(|decoy| unsafe { decoy.copy().as_mut() })
            {
                decoy.m_upDir = player.m_upDir;
                let mut dir1 = Vector3::ZERO;
                let mut dir2 = Vector3::ZERO;
                let mut vector_store = [Vector3::ZERO; 8];
                let angles =
                    unsafe { Vector3::new(0., (*player.get_angles(&mut vector_store[0])).y, 0.) };

                if vel.x.abs() == 0. && vel.y.abs() == 0. {
                    unsafe {
                        (server.decoy_set_orientation)(decoy, &mut vector_store, &angles, &angles);
                        (server.decoy_set_state)(decoy, 0);
                    }
                } else if !player.m_sliding {
                    // maybe sliding down a wall? or just wall hanging
                    if player.m_wallHanging
                        || player.m_upDir.z < 1.0
                            && (vel.y.powi(2) + vel.x.powi(2) <= player.m_upDir.z.powi(2))
                    {
                        unsafe {
                            (server.direction_to_angles)(&player.m_upDir, &mut dir1, &mut dir2);
                            (server.decoy_set_orientation)(
                                decoy,
                                &mut vector_store,
                                &angles,
                                &angles,
                            );
                            (server.decoy_set_state)(decoy, 3);
                        }
                    } else {
                        // wallrun
                        if player.m_upDir.z < 1.0 {
                            let normal_direction = 1. / (vel.x.powi(2) + vel.y.powi(2)).sqrt();

                            // let mut mystery_thing = Vector3::ZERO;
                            unsafe {
                                (server.direction_to_angles)(
                                    &Vector3::new(
                                        vel.x * normal_direction,
                                        vel.x * normal_direction,
                                        0.,
                                    ),
                                    &mut dir1,
                                    &mut dir2,
                                );
                                (server.decoy_set_orientation)(
                                    decoy,
                                    &mut vector_store,
                                    &angles,
                                    &angles,
                                );
                                (server.decoy_set_state)(decoy, 7);
                            }
                            // puVar6 = FUN_00556080((playerBoss->self).pl.self.currentClass);
                            // (ent->self).m_wallrunJumpStrength =
                            //      *(float *)(puVar6 + 0x230) + *(float *)(puVar6 + 0x22c);
                        } else if player.m_duckState == 2 {
                            let side_move =
                                (player.m_sideMove + player.m_forwardMove).clamp(0., 1.0);
                            if player.m_sideMove <= player.m_forwardMove.abs() {
                                // TODO: this has some magical variable which isn't set but I called it aaaaaaaaaaaaaaaaaaaaa
                                if side_move < 1. {
                                    unsafe { (server.decoy_set_state)(decoy, 2) };
                                } else {
                                    unsafe { (server.decoy_set_state)(decoy, 6) };
                                }
                            } else {
                                unsafe { (server.decoy_set_state)(decoy, 0xd) };
                            }
                            unsafe {
                                (server.direction_to_angles)(&player.m_upDir, &mut dir1, &mut dir2);
                                (server.decoy_set_orientation)(
                                    decoy,
                                    &mut vector_store,
                                    &angles,
                                    &angles,
                                )
                            };
                        } else if lookup_ent(player.m_activeZipline, server).is_none()
                            && !unsafe { (server.is_in_some_busy_interaction)(player) }
                        {
                            if !player.m_fIsSprinting {
                                let side_move =
                                    (player.m_sideMove + player.m_forwardMove).clamp(0., 1.0);
                                if player.m_sideMove <= player.m_forwardMove.abs() {
                                    // TODO: this has some magical variable which isn't set but I called it bbbbbbbbb
                                    if side_move < 1. {
                                        unsafe { (server.decoy_set_state)(decoy, 1) };
                                    } else {
                                        unsafe { (server.decoy_set_state)(decoy, 10) };
                                    }
                                } else {
                                    unsafe { (server.decoy_set_state)(decoy, 0xc) };
                                }
                            } else {
                                unsafe { (server.decoy_set_state)(decoy, 0xc) };
                            }
                            unsafe {
                                (server.direction_to_angles)(&player.m_upDir, &mut dir1, &mut dir2);
                                (server.decoy_set_orientation)(
                                    decoy,
                                    &mut vector_store,
                                    &angles,
                                    &angles,
                                )
                            };
                        } else if decoy.m_jumpTime < globals.curTime {
                            // puVar6 = GetPlayerClass((playerBoss->self).pl.self.currentClass);
                            // FUN_005d30c0();
                            // xVel = (*(float *)(puVar6 + 0x104) - *(float *)(puVar6 + 0x100)) * extraout_XMM0_Da +
                            //        *(float *)(puVar6 + 0x100);
                            // if (0.0 <= absVel.z) {
                            //   yVel = (absVel.z * absVel.z) / (*(float *)(DAT_00c38f68 + 0x58) * 2.0);
                            // }
                            // else {
                            //   yVel = 0.0;
                            // }
                            // fVar15 = xVel * *(float *)(DAT_00c08968 + 0x58);
                            // if (fVar15 + yVel < xVel) {
                            //   absVel.z = 0.0;
                            //   fVar15 = xVel;
                            // }
                            // xVel = absVel.z;
                            // yVel = sqrtf(*(float *)(DAT_00c38f68 + 0x58) * 2.0 * fVar15);

                            // FUN_006ee0c0(&local_90,(Vector3 *)&local_a0);
                            // xVel = SQRT(absVel.y * absVel.y + absVel.x * absVel.x);
                            // absVel.y = local_a0.y * xVel;
                            // absVel.x = local_a0.x * xVel;
                            decoy.m_jumpHeight = 100.;
                            decoy.m_jumpTime = globals.curTime + 0.016666668;
                            unsafe { (server.decoy_set_state)(decoy, 4) };
                        }
                    }
                } else if decoy.m_slideEndTime < globals.curTime {
                    unsafe {
                        // (server.decoy_set_modifiers)(decoy, 0x16);
                        (server.decoy_set_state)(decoy, 8);
                    };
                    // FUN_006ee0c0(&local_90, &newVel);
                    decoy.m_slideEndTime = globals.curTime + 0.1; // was 2.0
                    decoy.m_curSpeed = (vel.x.powi(2) + vel.y.powi(2)).sqrt();
                }
            }
        }
    }
}

fn create_player_decoy_hook(
    unk1: usize,
    boss_player: *mut CPlayer,
    unk2: usize,
    unk3: usize,
) -> *mut CPlayerDecoy {
    let decoy = unsafe { CreatePlayerDecoy.call(unk1, boss_player, unk2, unk3) };

    if let Some(boss_player) = unsafe { boss_player.as_ref() } {
        PLUGIN
            .wait()
            .holoplus
            .holo_pilot_table
            .lock()
            .entry(get_player_index(boss_player))
            .or_default()
            .push(unsafe { UnsafeHandle::new(decoy) });
    }

    decoy
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "DecoySetState")]
fn decoy_set_state(decoy: Option<&mut CBaseEntity>, state: i32) -> Result<(), String> {
    let decoy: &mut CPlayerDecoy = decoy
        .ok_or_else(|| "missing decoy".to_owned())?
        .dynamic_cast_mut()
        .ok_or_else(|| "not a decoy".to_owned())?;

    unsafe {
        (SERVER_FUNCTIONS.wait().decoy_set_state)(decoy, state);
    };

    Ok(())
}
