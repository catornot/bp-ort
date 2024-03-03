use rrplug::{
    bindings::class_types::{client::SignonState, cplayer::CPlayer},
    high::vector::Vector3,
};
use std::mem::MaybeUninit;

use crate::{
    bindings::{
        Action, CBaseEntity, CGlobalVars, CUserCmd, EngineFunctions, ServerFunctions, TraceResults,
        ENGINE_FUNCTIONS, SERVER_FUNCTIONS,
    },
    interfaces::ENGINE_INTERFACES,
    navmesh::{bindings::dtQueryFilter, RECAST_DETOUR},
    utils::{client_command, iterate_c_array_sized},
};

use super::{BotData, BotWeaponState, SIMULATE_TYPE_CONVAR, TASK_MAP};

#[derive(Clone)]
pub struct CUserCmdHelper<'a> {
    pub globals: &'a CGlobalVars,
    pub angles: Vector3,
    pub cmd_num: u32,
    pub sv_funcs: &'a ServerFunctions,
    pub engine_funcs: &'a EngineFunctions,
}

impl<'a> CUserCmdHelper<'a> {
    pub fn new(
        globals: &'a CGlobalVars,
        angles: Vector3,
        cmd_num: u32,
        sv_funcs: &'a ServerFunctions,
        engine_funcs: &'a EngineFunctions,
    ) -> CUserCmdHelper<'a> {
        Self {
            globals,
            angles,
            cmd_num,
            sv_funcs,
            engine_funcs,
        }
    }

    pub fn construct_from_global(s: &Self) -> Self {
        s.clone()
    }
}

impl CUserCmd {
    pub fn new_basic_move(move_: Vector3, buttons: u32, helper: &CUserCmdHelper) -> Self {
        unsafe {
            // union access :pain:
            CUserCmd {
                move_,
                tick_count: **helper.globals.tick_count,
                frame_time: **helper.globals.absolute_frame_time,
                command_time: **helper.globals.cur_time,
                command_number: helper.cmd_num,
                world_view_angles: helper.angles,
                local_view_angles: Vector3::ZERO,
                attackangles: helper.angles,
                buttons,
                impulse: 0,
                weaponselect: 0,
                meleetarget: 0,
                camera_pos: Vector3::ZERO,
                camera_angles: Vector3::ZERO,
                tick_something: **helper.globals.tick_count as i32,
                dword90: **helper.globals.tick_count + 4,
                ..CUserCmd::init_default(helper.sv_funcs)
            }
        }
    }

    pub fn new_empty(helper: &CUserCmdHelper) -> Self {
        unsafe {
            // union access :pain:
            CUserCmd {
                tick_count: **helper.globals.tick_count,
                frame_time: **helper.globals.absolute_frame_time,
                command_time: **helper.globals.cur_time,
                command_number: helper.cmd_num,
                world_view_angles: helper.angles,
                local_view_angles: Vector3::ZERO,
                attackangles: helper.angles,
                impulse: 0,
                weaponselect: 0,
                meleetarget: 0,
                camera_pos: Vector3::ZERO,
                camera_angles: Vector3::ZERO,
                tick_something: **helper.globals.tick_count as i32,
                dword90: **helper.globals.tick_count + 4,
                ..CUserCmd::init_default(helper.sv_funcs)
            }
        }
    }
}

pub fn run_bots_cmds() {
    let sim_type = SIMULATE_TYPE_CONVAR.wait().get_value_i32();
    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let run_null_command = server_functions.run_null_command;
    let add_user_cmd_to_player = server_functions.add_user_cmd_to_player;
    // let set_base_time = server_functions.set_base_time;
    // let player_run_command = server_functions.player_run_command;
    // let move_helper = server_functions.move_helper.cast_const();
    let globals =
        unsafe { engine_functions.globals.as_mut() }.expect("globals were null for some reason");

    let helper = CUserCmdHelper::new(
        globals,
        Vector3::ZERO,
        0,
        server_functions,
        engine_functions,
    );

    let bot_tasks = unsafe { TASK_MAP.as_mut() };

    for (cmd, player) in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| **client.signon == SignonState::FULL)
            .filter(|(_, client)| **client.fake_player)
            .filter_map(|(i, client)| {
                Some((
                    player_by_index((i + 1) as i32).as_mut()?,
                    **client.edict as usize,
                ))
            })
            .filter_map(|(p, edict)| {
                let data = bot_tasks.get_mut(edict)?;
                data.edict = edict as u16;
                Some((
                    get_cmd(p, &helper, data.sim_type.unwrap_or(sim_type), data)?,
                    p,
                ))
            }) // can collect here to stop the globals from complaning about mutability
    } {
        unsafe {
            add_user_cmd_to_player(
                player, &cmd, 1, // was amount
                1, // was amount
                0, // was amount as u32, seams like it was causing the dropped packets spam but also it was stoping the bots from going faster?
                0,
            );

            // let frametime = **globals.frametime;
            // let cur_time = **globals.cur_time;

            // set_base_time(player, cur_time);

            // run_null_command(player);
            // player_run_command(player, &mut cmd, move_helper);
            // *player.latest_command_run.get_inner_mut() = cmd.command_number;
            // (server_functions.set_last_cmd)(
            //     (player as *mut _ as *mut CUserCmd).offset(0x20a0).cast(),
            //     &mut cmd,
            // );
            // #[allow(invalid_reference_casting)] // tmp
            // {
            //     *((globals.frametime.get_inner() as *const f32).cast_mut()) = frametime;
            //     *((globals.cur_time.get_inner() as *const f32).cast_mut()) = cur_time;
            // }

            run_null_command(player); // doesn't really work?
        }
    }
}

pub(super) fn get_cmd(
    player: &mut CPlayer,
    helper: &CUserCmdHelper,
    sim_type: i32,
    local_data: &mut BotData,
) -> Option<CUserCmd> {
    let mut v = Vector3::default();
    let player_by_index = helper.sv_funcs.get_player_by_index;
    let helper = unsafe {
        CUserCmdHelper {
            angles: *(helper.sv_funcs.eye_angles)(player, &mut v),
            cmd_num: **player.rank as u32,
            ..CUserCmdHelper::construct_from_global(helper)
        }
    };

    let mut cmd = Some(match sim_type {
        _ if unsafe { (helper.sv_funcs.is_alive)(player) == 0 } => {
            local_data.weapon_state = BotWeaponState::ApReady;

            unsafe {
                client_command(
                    local_data.edict,
                    "CC_RespawnPlayer Pilot\0".as_ptr() as *const i8,
                )
            };

            CUserCmd::new_empty(&helper)
        }
        1 => {
            if unsafe { (helper.sv_funcs.is_on_ground)(player) } != 0 {
                CUserCmd::new_basic_move(Vector3::new(0., 0., 1.), Action::Jump as u32, &helper)
            } else {
                CUserCmd::new_basic_move(Vector3::new(0., 0., 0.), Action::Duck as u32, &helper)
            }
        }
        2 => {
            let origin = unsafe { *player.get_origin(&mut v) };

            let target = match local_data.counter {
                0 => Vector3::new(-528., 13., 2.),
                1 => Vector3::new(-592., -1401., 2.),
                2 => Vector3::new(-500., -1000., 2.),
                3 => Vector3::new(-400., -0., 2.),
                _ => {
                    local_data.counter = 0;
                    Vector3::new(-528., 13., 2.)
                }
            };

            let mut cmd = CUserCmd::new_basic_move(
                Vector3::new(1., 0., 0.),
                Action::Forward as u32 | Action::Walk as u32 | Action::Duck as u32,
                &helper,
            );

            if ((origin.x - target.x).powi(2) * (origin.y - target.y).powi(2)).sqrt() < 100. {
                local_data.counter += 1;
            }

            let diff = target - origin;
            cmd.world_view_angles.y = diff.y.atan2(diff.x).to_degrees();

            cmd
        }
        3 => unsafe {
            let origin = *player.get_origin(&mut v);
            let counter = &mut local_data.counter;
            let target = match player_by_index(1).as_mut() {
                Some(player)
                    if helper
                        .engine_funcs
                        .client_array
                        .as_mut()
                        .map(|client| !**client.fake_player)
                        .unwrap_or_default() =>
                {
                    *player.get_origin(&mut v)
                }
                _ => Vector3::ZERO,
            };

            let mut cmd = CUserCmd::new_basic_move(
                Vector3::new(1., 0., 0.),
                Action::Forward as u32 | Action::Speed as u32,
                &helper,
            );

            let distance =
                (origin.x - target.x).powi(2) as f64 * (origin.y - target.y).powi(2) as f64;

            if distance < 810000. {
                cmd.buttons = Action::Melee as u32;
                cmd.move_.x = 0.;
                *counter = 0;
            } else if distance > 625000000. {
                if *counter < 50 {
                    *counter += 1;
                } else {
                    *counter += 1;

                    if *counter > 200 {
                        *counter = 0;
                    }

                    let can_jump = *counter % 5 == 0;

                    if (helper.sv_funcs.is_on_ground)(player) != 0 && can_jump {
                        cmd.buttons |= Action::Jump as u32;
                    }
                    cmd.buttons |= Action::Duck as u32;
                }
            } else {
                cmd.buttons |= Action::Duck as u32;
            }

            let diff = target - origin;
            cmd.world_view_angles.y = diff.y.atan2(diff.x) * 180. / std::f32::consts::PI;

            cmd
        },
        4 | 5 => {
            let origin = unsafe { *player.get_origin(&mut v) };
            let team = unsafe { **player.team };

            let target = unsafe {
                find_closest_player(origin, team, &helper).map(|(player, should_shoot)| {
                    ((*player.get_origin(&mut v), player), should_shoot)
                })
            };

            let mut cmd = CUserCmd::new_basic_move(Vector3::ZERO, Action::Zoom as u32, &helper);

            if sim_type == 5 && target.is_some() {
                'ifstmt: {
                    cmd.move_ = Vector3::new(1., 0., 0.);
                    cmd.buttons |= Action::Forward as u32 | Action::Walk as u32;

                    let Some((ref target, _)) = target else {
                        break 'ifstmt;
                    };

                    let is_titan = unsafe { (helper.sv_funcs.is_titan)(player) };

                    if (!is_titan
                        && (origin.x - target.0.x).powi(2) * (origin.y - target.0.y).powi(2)
                            < 81000.
                        && (origin.z - target.0.z).abs() < 50.)
                        || (is_titan
                            && (origin.x - target.0.x).powi(2) * (origin.y - target.0.y).powi(2)
                                < 810000.
                            && (origin.z - target.0.z).abs() < 200.)
                    {
                        cmd.buttons |= Action::Melee as u32;
                    };

                    if is_titan && local_data.counter % 4 == 0 {
                        cmd.buttons |= Action::Dodge as u32;
                    }
                }
            }

            if let Some(((target, target_player), should_shoot)) = target {
                cmd.buttons |= if should_shoot {
                    Action::Attack as u32
                } else {
                    0
                };

                let target = if let Some(titan) =
                    unsafe { (helper.sv_funcs.get_pet_titan)(player).as_ref() }
                {
                    let titan_pos = unsafe {
                        *(helper.sv_funcs.get_origin)(
                            (titan as *const CBaseEntity).cast::<CPlayer>(),
                            &mut v,
                        )
                    };

                    if unsafe { view_rate(&helper, titan_pos, origin, player) } >= 1.0 {
                        if (origin.x - titan_pos.x).powi(2) * (origin.y - titan_pos.y).powi(2)
                            < 81000.
                        {
                            cmd.buttons |= Action::Use as u32;
                        }
                        titan_pos
                    } else {
                        target
                    }
                } else {
                    target
                };

                let diff = target - origin;
                let angley = diff.y.atan2(diff.x) * 180. / std::f32::consts::PI;
                let anglex = diff.z.atan2((diff.x.powi(2) + diff.y.powi(2)).sqrt()) * 180.
                    / std::f32::consts::PI;

                cmd.world_view_angles = Vector3::new(-anglex, angley, 0.);

                let enemy_is_titan = unsafe { (helper.sv_funcs.is_titan)(target_player) };
                let is_titan = unsafe { (helper.sv_funcs.is_titan)(player) };
                match (local_data.weapon_state, enemy_is_titan, is_titan) {
                    (BotWeaponState::ApReady, true, false) => {
                        local_data.weapon_state = BotWeaponState::AtPrepare
                    }
                    (BotWeaponState::ApPrepare, _, false) => unsafe {
                        // (helper.sv_funcs.set_weapon_by_slot)(
                        //     (player as *const _ as *const c_void).offset(0x12f8), // offset to inventory
                        //     "0\0".as_ptr() as *const i8,
                        // );
                        (helper.sv_funcs.replace_weapon)(
                            player,
                            "mp_weapon_rspn101\0".as_ptr() as *const i8,
                            std::ptr::null(),
                            std::ptr::null(),
                        );
                        // TODO: look at how givecurrentammo works

                        local_data.weapon_state = BotWeaponState::ApReady;
                    },
                    (BotWeaponState::AtReady, false, false) => {
                        local_data.weapon_state = BotWeaponState::ApPrepare
                    }
                    (BotWeaponState::AtPrepare, _, false) => unsafe {
                        (helper.sv_funcs.replace_weapon)(
                            player,
                            "mp_weapon_defender\0".as_ptr() as *const i8,
                            std::ptr::null(),
                            std::ptr::null(),
                        );

                        local_data.weapon_state = BotWeaponState::AtReady;
                    },
                    (bot_weapon, _, true)
                        if bot_weapon != BotWeaponState::TitanReady
                            || bot_weapon != BotWeaponState::TitanPrepare =>
                    {
                        local_data.weapon_state = BotWeaponState::TitanPrepare
                    }
                    (BotWeaponState::TitanReady, _, false) => {
                        local_data.weapon_state = BotWeaponState::ApPrepare
                    }
                    (BotWeaponState::TitanPrepare, _, true) => unsafe {
                        (helper.sv_funcs.replace_weapon)(
                            player,
                            "mp_titanweapon_sniper\0".as_ptr() as *const i8,
                            std::ptr::null(),
                            std::ptr::null(),
                        );

                        local_data.weapon_state = BotWeaponState::TitanReady;
                    },
                    _ => {}
                }

                if is_titan {
                    use super::TitanClass as TC;
                    cmd.buttons |= match (local_data.counter, local_data.titan) {
                        (_, TC::Scorch) => {
                            Action::OffHand0 as u32
                                | Action::OffHand1 as u32
                                | Action::OffHand2 as u32
                                | Action::OffHand3 as u32
                                | Action::OffHand4 as u32
                        }
                        (1, TC::Ronin | TC::Ion) => 0,
                        (2, TC::Legion) => 0,
                        (0, _) => Action::OffHand0 as u32,
                        (1, _) => Action::OffHand1 as u32,
                        (2, _) => Action::OffHand2 as u32,
                        (3, _) => Action::OffHand3 as u32,
                        (4, _) => {
                            local_data.counter = 0;
                            Action::OffHand4 as u32
                        }
                        _ => {
                            local_data.counter = 0;
                            0
                        }
                    };
                    local_data.counter += 1;
                }

                // unsafe {
                //     (helper.engine_funcs.render_line)(
                //         &origin,
                //         &target,
                //         Color {
                //             _color: [255, 0, 0, 255],
                //         },
                //         true,
                //     );
                // }
            } else {
                cmd.buttons = Action::Reload as u32;

                cmd.world_view_angles.x = 0.;
            }

            cmd
        }
        6 => unsafe {
            dbg!("befor crash");

            // server_command(to_c_string!(const "echo test\0").as_ptr());
            client_command(local_data.edict, "say test\0".as_ptr() as *const i8);

            CUserCmd::new_basic_move(
                Vector3::new(1., 0., 0.),
                Action::Forward as u32 | Action::Walk as u32,
                &helper,
            )
        },
        7 => 'end: {
            let dt_funcs = RECAST_DETOUR.wait();
            let debug = ENGINE_INTERFACES.wait().debug_overlay;
            let Some(nav) = local_data.nav_query.as_mut() else {
                log::warn!("null nav");
                break 'end CUserCmd::new_empty(&helper);
            };

            log::info!("query: {:?}", nav);

            let mut filter: dtQueryFilter = unsafe { MaybeUninit::zeroed().assume_init() };
            filter.m_areaCost = [
                1621.6901, 1274.1852, 1698.9136, 1158.3501, 1814.7485, 2123.6418, 0.0, 0.0,
                3243.3801, 2123.6418, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2123.6418, 0.0,
                0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
            ]; // magic numbers

            // should move it to local_data
            // let filter = unsafe { *(dt_funcs.navmesh_maybe_init_filter)(filter.as_mut_ptr()) };

            const GOAL: Vector3 = Vector3::new(-207.0, -1750.0, 1.0);
            const START: Vector3 = Vector3::new(-214.0, -771.0, 1.0);
            const EXTENTS: Vector3 = Vector3::new(100.0, 100.0, 136.0);

            let mut v = Vector3::ZERO;

            let origin = unsafe { *player.get_center_position(&mut v) };
            let target = unsafe {
                *(helper.sv_funcs.get_player_by_index)(1)
                    .as_mut()
                    .unwrap_or(player)
                    .get_center_position(&mut v)
            };

            let mut ref_start = 0;
            let mut start = Vector3::ZERO;
            let mut ref_goal = 0;
            let mut goal = Vector3::ZERO;

            let status = unsafe {
                (dt_funcs.dtNavMeshQuery__findNearestPoly)(
                    nav,
                    &target,
                    &EXTENTS,
                    // std::ptr::null(),
                    &filter,
                    &mut ref_goal,
                    &mut goal,
                )
                .eq(&0x40000000)
                .then(|| {
                    (dt_funcs.dtNavMeshQuery__findNearestPoly)(
                        nav,
                        &origin,
                        &EXTENTS,
                        // std::ptr::null(),
                        &filter,
                        &mut ref_start,
                        &mut start,
                    )
                    .eq(&0x40000000)
                })
                .unwrap_or(false)
            };

            unsafe { debug.AddLineOverlay(&start, &goal, 255, 0, 0, true, 2.) };
            unsafe { debug.AddLineOverlay(&target, &origin, 255, 0, 0, true, 10.) };

            if !status || ref_goal == 0 || ref_start == 0 {
                log::warn!(
                    "failed to find nearest poly, with goal {} start {}",
                    ref_goal,
                    ref_start
                );
                break 'end CUserCmd::new_empty(&helper);
            }
            log::info!(
                "found nearest poly, with goal {} {:?} start {} {:?}",
                ref_goal,
                goal,
                ref_start,
                start
            );

            let mut path = [0; 100];
            let mut path_size = 0;
            let unk: i64 = 0;

            unsafe {
                (dt_funcs.dtNavMeshQuery__findPath)(
                    nav,
                    ref_start,
                    ref_goal,
                    &start,
                    &goal,
                    // std::ptr::null(),
                    &filter,
                    path.as_mut_ptr(),
                    (&unk as *const i64).cast(),
                    &mut path_size,
                    path.len() as i32,
                )
            };

            if path_size == 0 {
                log::warn!("failed to find path");
                break 'end CUserCmd::new_empty(&helper);
            }

            log::info!("{path_size} {path:?}");

            // for polyref in path.iter().cloned().array_chunks::<2>() {

            // }
            // ENGINE_INTERFACES.wait().debug_overlay.AddLineOverlay(, , , , , , )

            CUserCmd::new_empty(&helper)
        }
        _ => CUserCmd::new_empty(&helper),
    })?;

    unsafe {
        **player.rank += 1; // using this for command number
        cmd.command_number = **player.rank as u32;
    }

    Some(cmd)
}

unsafe fn find_closest_player<'a>(
    pos: Vector3,
    team: i32,
    helper: &CUserCmdHelper,
) -> Option<(&'a mut CPlayer, bool)> {
    let mut v = Vector3::ZERO;
    if let Some(target) = unsafe {
        (0..32)
            .filter_map(|i| (helper.sv_funcs.get_player_by_index)(i + 1).as_mut())
            .filter(|player| **player.team != team && **player.team != 0)
            .filter(|player| (helper.sv_funcs.is_alive)(*player) != 0)
            .map(|player| (*player.get_origin(&mut v), player))
            .map(|(target, player)| (view_rate(helper, pos, target, player), player))
            .find(|(dist, _)| *dist >= 1.0)
            .map(|(_, player)| player)
    } {
        return Some((target, true));
    }

    let mut targets = unsafe {
        (0..32)
            .filter_map(|i| (helper.sv_funcs.get_player_by_index)(i + 1).as_mut())
            .filter(|player| **player.team != team && **player.team != 0)
            .filter(|player| (helper.sv_funcs.is_alive)(*player) != 0)
            .map(|player| (*player.get_origin(&mut v), player))
            // .map(|(target, player)| (view_rate(helper, pos, target, player), player))
            .map(|(target, player)| (distance_rate(pos, target), player))
            // .filter(|(target, _)| view_rate(helper, pos, *target) == 1000)
            .collect::<Vec<(i64, &mut CPlayer)>>()
    };

    targets.sort_by(|(dis1, _), (dis2, _)| dis1.cmp(dis2));
    // targets.into_iter().last().map(|(_, player)| player) // for view_rate
    targets
        .into_iter()
        .next()
        .map(|(_, player)| (player, false)) // for distance_rate
}

#[allow(unused)]
unsafe fn view_rate(
    helper: &CUserCmdHelper,
    v1: Vector3,
    v2: Vector3,
    player: *mut CPlayer,
) -> f32 {
    const POS_OFFSET: Vector3 = Vector3::new(0., 0., 50.);

    let (v1, v2) = (v1 + POS_OFFSET, v2 + POS_OFFSET);

    // let id = ((player.offset(0x30) as usize) & 0xffff);
    // let id = (helper.sv_funcs.base.offset(0xb6ab58) as usize + id * 30) as i8 * -1;

    const TRACE_MASK_SHOT: i32 = 1178615859;
    const TRACE_MASK_SOLID_BRUSHONLY: i32 = 16907;

    let mut result: MaybeUninit<TraceResults> = MaybeUninit::zeroed();
    (helper.sv_funcs.trace_line_simple)(
        &v2,
        &v1,
        TRACE_MASK_SHOT as i8,
        0,
        0,
        0,
        0,
        result.as_mut_ptr(),
    );
    let result = result.assume_init();

    // (result.fraction * 1000.) as i64
    result.fraction
}

#[allow(unused)]
fn distance_rate(pos: Vector3, target: Vector3) -> i64 {
    ((pos.x - target.x).powi(2) as f64 * (pos.y - target.y).powi(2) as f64) as i64
}
