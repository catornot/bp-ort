use rrplug::{
    bindings::class_types::{client::SignonState, cplayer::CPlayer},
    high::vector::Vector3,
};

use crate::{
    bindings::{
        Action, CGlobalVars, CUserCmd, EngineFunctions, ServerFunctions, ENGINE_FUNCTIONS,
        SERVER_FUNCTIONS,
    },
    utils::iterate_c_array_sized,
};

use super::SIMULATE_TYPE_CONVAR;

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
}

pub fn run_bots_cmds() {
    let sim_type = SIMULATE_TYPE_CONVAR.wait().get_value_i32();
    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let run_null_command = server_functions.run_null_command;
    let add_user_cmd_to_player = server_functions.add_user_cmd_to_player;
    // let set_base_time = server_functions.set_base_time;
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

    for (cmd, player) in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| **client.signon == SignonState::FULL)
            .filter(|(_, client)| **client.fake_player)
            .filter_map(|(i, _)| (player_by_index((i + 1) as i32).as_mut()))
            .filter_map(|p| Some((get_cmd(p, &helper, sim_type)?, p)))
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
            // // (server_functions.set_last_cmd)(
            // //     (player as *mut _ as *mut CUserCmd).offset(0x20a0).cast(),
            // //     &cmd.command_number as *const _ as *mut CUserCmd,
            // // );
            // **globals.frametime = frametime;
            // **globals.cur_time = cur_time;

            run_null_command(player); // doesn't really work?
        }
    }
}

pub fn get_cmd(player: &mut CPlayer, helper: &CUserCmdHelper, sim_type: i32) -> Option<CUserCmd> {
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
        _ if unsafe { **player.health } <= 0 => {
            let gen = &mut unsafe { **player.generation };
            if *gen == 0 {
                *gen = 1;
                CUserCmd::new_basic_move(Vector3::new(0., 0., 0.), Action::Jump as u32, &helper)
            } else {
                *gen = 0;
                return None;
            }
            // auto respawn
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

            let gen = &mut unsafe { **player.generation };
            let target = match gen {
                0 => Vector3::new(-528., 13., 2.),
                1 => Vector3::new(-592., -1401., 2.),
                2 => Vector3::new(-500., -1000., 2.),
                3 => Vector3::new(-400., -0., 2.),
                _ => {
                    *gen = 0;
                    Vector3::new(-528., 13., 2.)
                }
            };

            let mut cmd = CUserCmd::new_basic_move(
                Vector3::new(1., 0., 0.),
                Action::Forward as u32 | Action::Walk as u32 | Action::Duck as u32,
                &helper,
            );

            if (origin.x - target.x).powi(2) * (origin.y - target.y).powi(2) < 810000. {
                *gen += 1;
            }

            let diff = target - origin;
            cmd.world_view_angles.y = diff.y.atan2(diff.x) * 180. / std::f32::consts::PI;

            cmd
        }
        3 => unsafe {
            let origin = *player.get_origin(&mut v);
            let gen = &mut **player.generation;
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
                cmd.buttons = Action::Null as u32;
                cmd.move_.x = 0.;
                *gen = 0;
            } else if distance > 625000000. {
                if *gen < 50 {
                    *gen += 1;
                } else {
                    *gen += 1;

                    if *gen > 200 {
                        *gen = 0;
                    }

                    let can_jump = *gen % 5 == 0;

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
        4 => {
            let origin = unsafe { *player.get_origin(&mut v) };
            let team = unsafe { **player.team };

            let mut targets = unsafe {
                (0..32)
                    .filter_map(|i| player_by_index(i + 1).as_mut())
                    .filter(|player| **player.team != team)
                    .map(|player| (*player.get_origin(&mut v), player))
                    .map(|(target, player)| {
                        (
                            player,
                            ((origin.x - target.x).powi(2) as f64
                                * (origin.y - target.y).powi(2) as f64)
                                as i64,
                        )
                    })
                    .collect::<Vec<(&mut CPlayer, i64)>>()
            };
            targets.sort_by(|(_, dis1), (_, dis2)| dis1.cmp(dis2));

            let target = targets
                .first()
                .map(|(player, _)| unsafe { *player.get_origin(&mut v) })
                .unwrap_or(origin);

            let mut cmd = CUserCmd::new_basic_move(
                Vector3::ZERO,
                Action::Attack as u32 | Action::Zoom as u32,
                &helper,
            );

            let diff = target - origin;
            let angley = diff.y.atan2(diff.x) * 180. / std::f32::consts::PI;
            let anglex = diff.z.atan2(diff.x) * 180. / std::f32::consts::PI;

            cmd.world_view_angles = Vector3::new(-anglex, angley, 0.);

            cmd
        }
        5 => CUserCmd::new_basic_move(
            Vector3::new(-1., -0., 0.),
            Action::Back as u32 | Action::Walk as u32,
            &helper,
        ),
        6 => CUserCmd::new_basic_move(
            Vector3::new(1., 0., 0.),
            Action::Forward as u32 | Action::Walk as u32,
            &helper,
        ),
        _ => None?,
    })?;

    unsafe {
        **player.rank += 1; // using this for command number
        cmd.command_number = **player.rank as u32;
    }

    Some(cmd)
}
