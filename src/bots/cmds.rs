use itertools::Itertools;
use rand::{thread_rng, Rng};
use rrplug::{
    bindings::class_types::{client::SignonState, cplayer::CPlayer},
    high::{squirrel::call_sq_function, vector::Vector3, UnsafeHandle},
    mid::squirrel::{SQFUNCTIONS, SQVM_SERVER},
    prelude::EngineToken,
};
use std::mem::MaybeUninit;

use crate::{
    bindings::{
        Action, CBaseEntity, CGlobalVars, CTraceFilterSimple, CUserCmd, EngineFunctions, Ray,
        ServerFunctions, TraceResults, VectorAligned, ENGINE_FUNCTIONS, SERVER_FUNCTIONS,
    },
    interfaces::ENGINE_INTERFACES,
    navmesh::{Hull, RECAST_DETOUR},
    utils::{get_net_var, iterate_c_array_sized},
};

use super::{
    cmds_helper::CUserCmdHelper, cmds_utils::*, BotData, BOT_DATA_MAP, SIMULATE_TYPE_CONVAR,
};

const GROUND_OFFSET: Vector3 = Vector3::new(0., 0., 20.);
const BOT_VISON_RANGE: f32 = 3000.;
const BOT_PATH_NODE_RANGE: f32 = 50.;
const BOT_PATH_RECAL_RANGE: f32 = 600.;

pub(super) fn get_cmd(
    player: &'static mut CPlayer,
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

    {
        let desired_hull = if unsafe { (helper.sv_funcs.is_titan)(player) } {
            Hull::Titan
        } else {
            Hull::Human
        };
        if Some(desired_hull) != local_data.nav_query.as_ref().map(|q| q.hull) {
            if let Some(ref mut query) = local_data.nav_query {
                _ = query.switch_query(desired_hull);
            }
        }
    }

    let command_number = unsafe {
        **player.rank += 1; // using this for command number
        **player.rank as u32
    };

    let mut cmd = Some(match sim_type {
        _ if unsafe { (helper.sv_funcs.is_alive)(player) == 0 } => {
            if let Some(query) = local_data.nav_query.as_mut() {
                query.path_points.clear()
            }

            let sqvm = SQVM_SERVER
                .get(unsafe { EngineToken::new_unchecked() })
                .borrow();
            if let Some(sqvm) = sqvm.as_ref() {
                call_sq_function::<(), _>(
                    *sqvm,
                    SQFUNCTIONS.server.wait(),
                    "CodeCallBack_Test",
                    unsafe { UnsafeHandle::new(&*player) },
                )
                .unwrap_or_default();
            }

            CUserCmd::new_empty(&helper)
        }
        1 | 12 => {
            local_data.counter += 1;
            if unsafe { (helper.sv_funcs.is_on_ground)(player) } != 0
                && local_data.counter / 10 % 4 == 0
            {
                CUserCmd::new_basic_move(Vector3::new(0., 0., 1.), Action::Jump as u32, &helper)
            } else {
                CUserCmd::new_basic_move(
                    Vector3::new(0., 0., 0.),
                    if sim_type == 12 {
                        Action::Attack
                    } else {
                        Action::Duck
                    } as u32,
                    &helper,
                )
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

            cmd.world_view_angles.y = look_at(origin, target).y;

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

            *player.angles.get_inner_mut() = cmd.world_view_angles;

            cmd
        },
        4..=7 => {
            let origin = unsafe { *player.get_origin(&mut v) };
            let team = unsafe { **player.team };

            let target = unsafe {
                find_player_in_view(
                    origin,
                    Some(*(helper.sv_funcs.view_angles)(player, &mut v)),
                    team,
                    &helper,
                )
                .map(|(player, should_shoot)| ((*player.get_origin(&mut v), player), should_shoot))
                .or_else(|| {
                    distance_iterator(
                        &origin,
                        enemy_player_iterator(team, &helper)
                            .chain(enemy_titan_iterator(&helper, team)),
                    )
                    .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
                    .map(|(_, player)| player)
                    .map(|player| ((*player.get_origin(&mut v), player), false))
                })
            };

            let mut cmd = CUserCmd::new_basic_move(Vector3::ZERO, 0, &helper);

            match (sim_type, &target) {
                (6, target) if target.is_none() || matches!(target, Some((_, false))) => {
                    if let Some(pet_titan) =
                        unsafe { (helper.sv_funcs.get_pet_titan)(player).as_ref() }
                    {
                        path_to_target(
                            &mut cmd,
                            local_data,
                            origin,
                            unsafe {
                                *(helper.sv_funcs.get_origin)(
                                    (pet_titan as *const CBaseEntity).cast(),
                                    &mut v,
                                )
                            },
                            local_data.should_recaculate_path,
                            &helper,
                        );
                    } else if let Some(((target_pos, target), _)) = target {
                        if path_to_target(
                            &mut cmd,
                            local_data,
                            origin,
                            *target_pos,
                            local_data.last_target_index
                                != unsafe { target.player_index.copy_inner() }
                                || local_data.should_recaculate_path,
                            &helper,
                        ) {
                            local_data.last_target_index =
                                unsafe { target.player_index.copy_inner() }
                        }
                    }

                    local_data.should_recaculate_path = false;
                }
                (7, vision) if vision.is_none() || matches!(vision, Some((_, false))) => {
                    _ = path_to_target(
                        &mut cmd,
                        local_data,
                        origin,
                        local_data.target_pos,
                        local_data.should_recaculate_path,
                        &helper,
                    );

                    local_data.should_recaculate_path = false;
                }
                (_, Some((_, _))) => {
                    cmd.move_ = Vector3::new(1., 0., 0.);
                    cmd.buttons |= Action::Forward as u32 | Action::Walk as u32;

                    local_data.should_recaculate_path = true;
                }
                _ => {}
            }

            if let Some(((target, target_player), should_shoot)) = target {
                cmd.buttons |= if should_shoot && is_timedout(local_data.last_shot, &helper, 0.8) {
                    Action::Zoom as u32
                        | (unsafe { helper.globals.frame_count.copy_inner() } / 2 % 4 != 0)
                            .then_some(Action::Attack as u32)
                            .unwrap_or_default()
                } else if should_shoot {
                    0
                } else {
                    local_data.last_shot = unsafe { helper.globals.cur_time.copy_inner() };
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

                    let (dis, ent) = unsafe { view_rate(&helper, titan_pos, origin, player, true) };
                    if dis >= 1.0 || ent == titan as *const CBaseEntity {
                        if (origin.x - titan_pos.x).powi(2) * (origin.y - titan_pos.y).powi(2)
                            < 81000.
                            && (unsafe { helper.globals.frame_count.copy_inner() } / 2 % 4 != 0)
                        {
                            cmd.world_view_angles = look_at(origin, titan_pos);
                            cmd.buttons |= Action::Use as u32;
                        }
                        titan_pos
                    } else {
                        target
                    }
                } else {
                    target
                };

                if should_shoot || sim_type == 5 {
                    let angles = look_at(origin, target);

                    let angles = {
                        let velocity = unsafe {
                            *(helper.sv_funcs.get_smoothed_velocity)(target_player, &mut v)
                        };
                        let length = (velocity.x.powi(2) + velocity.y.powi(2)).sqrt();

                        if length > 200. {
                            let mut rng = thread_rng();
                            let error_amount = length.sqrt() / 10f32;

                            Vector3 {
                                x: angles.x + error_amount * rng.gen_range(-2..=2) as f32,
                                y: angles.y + error_amount * rng.gen_range(-2..=2) as f32,
                                z: 0.,
                            }
                        } else {
                            angles
                        }
                    };

                    const CLAMP: f32 = 10.;

                    cmd.world_view_angles.x = angles.x;
                    cmd.world_view_angles.y = angles
                        .y
                        .is_finite()
                        .then(|| {
                            angles.y.clamp(
                                cmd.world_view_angles.y - CLAMP,
                                cmd.world_view_angles.y + CLAMP,
                            )
                        })
                        .unwrap_or(angles.y);
                }

                let enemy_is_titan = unsafe { (helper.sv_funcs.is_titan)(target_player) };
                let is_titan = unsafe { (helper.sv_funcs.is_titan)(player) };

                if (!is_titan
                    && (origin.x - target.x).powi(2) * (origin.y - target.y).powi(2) < 81000.
                    && (origin.z - target.z).abs() < 50.)
                    || (is_titan
                        && (origin.x - target.x).powi(2) * (origin.y - target.y).powi(2) < 850000.
                        && (origin.z - target.z).abs() < 200.)
                {
                    cmd.buttons |= Action::Melee as u32;
                };

                if is_titan && local_data.counter % 4 == 0 {
                    cmd.buttons |= Action::Dodge as u32;
                }

                match (enemy_is_titan, is_titan) {
                    (true, true) => cmd.weaponselect = 0, // switch to default,
                    (true, false) => cmd.weaponselect = 1,
                    (false, true) => cmd.weaponselect = 0, // switch to default,
                    (false, false) => cmd.weaponselect = 0, // switch to default,
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
                        (4, _) if should_shoot => {
                            local_data.counter = 0;
                            Action::OffHand4 as u32 // core
                        }
                        _ => {
                            local_data.counter = 0;
                            0
                        }
                    };
                    local_data.counter += 1;
                }
            } else {
                cmd.buttons = Action::Reload as u32;

                cmd.world_view_angles.x = 0.;
            }

            if is_timedout(local_data.next_check, &helper, 10f32)
                && get_net_var(player, c"goalState", 124, helper.sv_funcs) == Some(2)
            {
                log::info!("bot calling titan down");

                let sqvm = SQVM_SERVER
                    .get(unsafe { EngineToken::new_unchecked() })
                    .borrow();
                if let Some(sqvm) = sqvm.as_ref() {
                    call_sq_function::<(), _>(
                        *sqvm,
                        SQFUNCTIONS.server.wait(),
                        "CodeCallback_ClientCommand",
                        (
                            unsafe { UnsafeHandle::new(&*player) },
                            ["ClientCommand_RequestTitan".to_owned()],
                        ),
                    )
                    .unwrap_or_default();
                }

                local_data.next_check = unsafe { helper.globals.cur_time.copy_inner() }
            }

            cmd.camera_angles = cmd.world_view_angles;

            cmd
        }
        13 | 14 => 'end: {
            let mut cmd = CUserCmd::new_empty(&helper);

            let origin = unsafe { *player.get_origin(&mut v) };
            let team = unsafe { **player.team };
            let mut v = Vector3::ZERO;

            let maybe_target = if sim_type == 13 {
                farthest_player(origin, team, &helper)
            } else {
                closest_player(origin, team, &helper)
            };

            let Some(target) = maybe_target else {
                break 'end cmd;
            };
            let target_pos = unsafe { *target.get_origin(&mut v) };

            if path_to_target(
                &mut cmd,
                local_data,
                origin,
                target_pos,
                local_data.last_target_index != unsafe { target.player_index.copy_inner() },
                &helper,
            ) {
                local_data.last_target_index = unsafe { target.player_index.copy_inner() }
            }
            cmd
        }
        15 => {
            let mut cmd = CUserCmd::new_empty(&helper);
            cmd.world_view_angles = helper.angles + Vector3::new(0., 10., 0.);

            local_data.counter += 1;
            if local_data.counter % 4 == 0 {
                cmd.buttons |= Action::Duck as u32;
            }

            cmd.weaponselect = 2;

            cmd
        }
        16 => {
            let mut cmd = CUserCmd::new_basic_move(
                Vector3::new(1.0, 0., 0.),
                Action::Forward as u32,
                &helper,
            );
            cmd.world_view_angles = helper.angles + Vector3::new(0., 10., 0.);

            cmd
        }
        17 => {
            let origin = unsafe { *player.get_origin(&mut v) };
            let team = unsafe { **player.team };

            let target = unsafe {
                find_player_in_view(
                    origin,
                    Some(*(helper.sv_funcs.view_angles)(player, &mut v)),
                    team,
                    &helper,
                )
                .map(|(player, should_shoot)| ((*player.get_origin(&mut v), player), should_shoot))
            };

            log::info!(
                "can see target {} at {:?}",
                target
                    .as_ref()
                    .map(|(_, can_see)| *can_see)
                    .unwrap_or(false),
                target.map(|((pos, _), _)| pos)
            );

            CUserCmd::new_empty(&helper)
        }
        18 => 'scope: {
            // battery yoinker
            let mut cmd =
                CUserCmd::new_basic_move(Vector3::new(1., 0., 0.), Action::Forward as u32, &helper);
            let origin = unsafe { *player.get_origin(&mut v) };
            let team = unsafe { **player.team };
            local_data.counter = local_data.counter.wrapping_add(1);

            if unsafe { player.titan_soul_being_rodeoed.copy_inner() } != -1 {
                log::info!(
                    "{} {}",
                    local_data.last_shot,
                    is_timedout(local_data.last_shot, &helper, 20.)
                );

                if is_timedout(local_data.last_shot, &helper, 10.)
                    && local_data.counter / 10 % 4 == 0
                {
                    cmd.buttons |= Action::Jump as u32 | Action::WeaponDiscard as u32;
                }
                break 'scope cmd;
            } else {
                local_data.last_shot = unsafe { helper.globals.cur_time.copy_inner() };
            }

            let is_team = move |player: &CPlayer| -> bool { unsafe { **player.team == team } };
            let maybe_rodeo_target = get_net_var(player, c"batteryCount", 191, helper.sv_funcs)
                .and_then(|value| value.eq(&0).then_some(()))
                .and_then(|_| {
                    distance_iterator(
                        &origin,
                        enemy_player_iterator(team, &helper)
                            .chain(enemy_titan_iterator(&helper, team))
                            .filter(|ent| unsafe { (helper.sv_funcs.is_titan)(*ent) }),
                    )
                    .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
                    .map(|(_, player)| unsafe { *player.get_origin(&mut v) })
                })
                .or_else(|| {
                    distance_iterator(
                        &origin,
                        player_iterator(&is_team, &helper)
                            .chain(titan_iterator(&is_team, &helper))
                            .filter(|ent| unsafe { (helper.sv_funcs.is_titan)(*ent) }),
                    )
                    .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
                    .map(|(_, player)| unsafe { *player.get_origin(&mut v) })
                });

            if let Some(rodeo_target) = maybe_rodeo_target {
                if distance(origin, rodeo_target) > 100. {
                    path_to_target(&mut cmd, local_data, origin, rodeo_target, false, &helper);
                } else if unsafe { (helper.sv_funcs.is_on_ground)(player) } != 0
                    && local_data.counter / 10 % 4 == 0
                {
                    cmd.buttons |= Action::Jump as u32;
                }
            } else {
                cmd.move_ = Vector3::ZERO;
            }

            cmd
        }
        19 => {
            let mut cmd = CUserCmd::new_empty(&helper);
            let origin = unsafe { *player.get_origin(&mut v) };

            if let Some(titan_pos) = unsafe { (helper.sv_funcs.get_pet_titan)(player).as_ref() }
                .map(|titan| unsafe {
                    *(helper.sv_funcs.get_origin)((titan as *const CBaseEntity).cast(), &mut v)
                })
            {
                path_to_target(
                    &mut cmd,
                    local_data,
                    origin,
                    titan_pos,
                    local_data.should_recaculate_path,
                    &helper,
                );

                if (origin.x - titan_pos.x).powi(2) * (origin.y - titan_pos.y).powi(2) < 81000.
                    && (unsafe { helper.globals.frame_count.copy_inner() } / 2 % 4 != 0)
                {
                    cmd.world_view_angles = look_at(origin, titan_pos);
                    cmd.buttons |= Action::Use as u32;
                }
            } else {
                cmd.world_view_angles.x = -90.;
                cmd.buttons |= Action::Duck as u32;
            }

            cmd
        }
        _ => CUserCmd::new_empty(&helper),
    })?;

    cmd.command_number = command_number;

    Some(cmd)
}
