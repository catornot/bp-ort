use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};

use crate::{
    bindings::{Action, CUserCmd},
    navmesh::Hull,
};

use super::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData, BotShared};

mod basic_combat;
mod battery_yoinker;
mod hardpoint;
mod slide_hopper;

pub fn reset_on_new_game() {
    hardpoint::reset_hardpoint(None, None);
}

pub(super) fn get_cmd(
    player: &mut CPlayer,
    helper: &CUserCmdHelper,
    sim_type: i32,
    local_data: &mut BotData,
    shared: &mut BotShared,
) -> Option<CUserCmd> {
    let mut v = Vector3::default();
    let player_by_index = helper.sv_funcs.get_player_by_index;
    let helper = unsafe {
        CUserCmdHelper {
            angles: *(helper.sv_funcs.eye_angles)(player, &mut v),
            cmd_num: player.m_rank as u32,
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

    let command_number = {
        player.m_rank += 1; // using this for command number
        player.m_rank as u32
    };

    let mut cmd = Some(match sim_type {
        _ if unsafe { (helper.sv_funcs.is_alive)(player) == 0 } => {
            if let Some(query) = local_data.nav_query.as_mut() {
                query.path_points.clear()
            }
            hardpoint::reset_hardpoint(Some(player), Some(local_data));

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
            let origin = player.m_vecAbsOrigin;

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

            player.m_localAngles = cmd.world_view_angles;

            cmd
        },
        4..=10 => basic_combat::basic_combat(player, &helper, sim_type, local_data, shared),
        13..=15 => 'end: {
            let mut cmd = CUserCmd::new_empty(&helper);

            let origin = unsafe { *player.get_origin(&mut v) };
            let team = player.m_iTeamNum;
            let mut v = Vector3::ZERO;

            let maybe_target = if sim_type == 13 {
                farthest_player(origin, team, &helper)
                    .map(|target| unsafe { (*target.get_origin(&mut v), target.pl.index) })
            } else if sim_type == 14 {
                closest_player(origin, team, &helper)
                    .map(|target| unsafe { (*target.get_origin(&mut v), target.pl.index) })
            } else {
                Some((local_data.target_pos, local_data.target_pos.x as i32))
            };

            let Some((target, last_index)) = maybe_target else {
                break 'end cmd;
            };

            if path_to_target(
                &mut cmd,
                local_data,
                origin,
                target,
                local_data.last_target_index != last_index,
                &helper,
            ) {
                local_data.last_target_index = last_index
            }
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
            let origin = player.m_vecAbsOrigin;
            let team = player.m_iTeamNum;

            let target = unsafe {
                find_player_in_view(
                    origin,
                    Some(*(helper.sv_funcs.view_angles)(player, &mut v)),
                    team,
                    &helper,
                    None,
                    None,
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
                .map(|titan| unsafe { *titan.get_origin(&mut v) })
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
                    && (helper.globals.frameCount / 2 % 4 != 0)
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
        20 => slide_hopper::slide_hopper(&helper, player, local_data),
        21 => {
            let mut cmd = CUserCmd::new_empty(&helper);
            let origin = unsafe { *player.get_origin(&mut v) };

            local_data.approach_range = Some(1.);
            _ = path_to_target(
                &mut cmd,
                local_data,
                origin,
                local_data.target_pos,
                local_data.should_recaculate_path,
                &helper,
            );

            local_data.should_recaculate_path = false;

            cmd
        }
        22 => {
            local_data.approach_range = None;
            local_data.target_pos = Vector3::new(10000., 10000., 10000.);

            CUserCmd::new_empty(&helper)
        }

        23 => {
            let mut cmd = CUserCmd::new_empty(&helper);
            let origin = unsafe { *player.get_origin(&mut v) };

            cmd.world_view_angles = look_at(origin, local_data.target_pos);

            cmd
        }
        30 => CUserCmd::new_basic_move(Vector3::ZERO, Action::OffHand0 as u32, &helper),
        31 => CUserCmd::new_basic_move(Vector3::ZERO, Action::OffHand1 as u32, &helper),
        32 => CUserCmd::new_basic_move(Vector3::ZERO, Action::OffHand2 as u32, &helper),
        33 => CUserCmd::new_basic_move(Vector3::ZERO, Action::OffHand3 as u32, &helper),
        34 => CUserCmd::new_basic_move(Vector3::ZERO, Action::OffHand4 as u32, &helper),
        _ => CUserCmd::new_empty(&helper),
    })?;

    cmd.command_number = command_number;

    Some(cmd)
}
