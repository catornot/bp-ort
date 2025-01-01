use rrplug::{
    bindings::class_types::cplayer::CPlayer,
    high::{squirrel::call_sq_function, UnsafeHandle},
    mid::squirrel::SQVM_SERVER,
    prelude::*,
};

use crate::{
    bindings::{Action, CBaseEntity, CUserCmd},
    navmesh::Hull,
};

use super::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData};

mod basic_combat;
mod battery_yoinker;

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
        4..=7 => basic_combat::basic_combat(player, &helper, sim_type, local_data),
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
        18 => battery_yoinker::battery_yoinker(&helper, player, local_data),
        19 => {
            // titan mounter
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
