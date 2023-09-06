use rrplug::{bindings::class_types::client::SignonState, high::vector::Vector3};
use std::ops::Deref;

use super::{SIMULATE_CONVAR, SIMULATE_TYPE_CONVAR};
use crate::{
    bindings::{
        Action, CGlobalVars, CUserCmd, ServerFunctions, ENGINE_FUNCTIONS, SERVER_FUNCTIONS,
    },
    utils::iterate_c_array_sized,
};

pub fn run_bots_cmds() {
    if SIMULATE_CONVAR.wait().get_value_i32() != 1 {
        return;
    }

    let cmd_type = SIMULATE_TYPE_CONVAR.wait().get_value_i32();

    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let run_null_command = server_functions.run_null_command;
    let add_user_cmd_to_player = server_functions.add_user_cmd_to_player;
    let eye_angles = server_functions.eye_angles;
    let globals =
        unsafe { engine_functions.globals.as_ref() }.expect("globals were null for some reason");

    for player in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| *client.signon.deref().deref() == SignonState::FULL)
            .filter(|(_, client)| **client.fake_player)
            .filter_map(|(i, _)| (player_by_index((i + 1) as i32).as_mut()))
    } {
        unsafe {
            let mut v = Vector3::ZERO;

            let angles = *eye_angles(player, &mut v);

            let helper =
                CUserCmdHelper::new(globals, angles, **player.rank as u32, server_functions);

            let mut cmds = match cmd_type {
                1 => [if (server_functions.is_on_ground)(player) != 0 {
                    CUserCmd::new_basic_move(Vector3::new(0., 0., 1.), Action::Jump as u32, &helper)
                } else {
                    CUserCmd::new_basic_move(Vector3::new(0., 0., 0.), Action::Duck as u32, &helper)
                }]
                .to_vec(),
                2 => {
                    let target = match player_by_index(1).as_mut() {
                        Some(player)
                            if engine_functions
                                .client_array
                                .as_mut()
                                .map(|client| !**client.fake_player)
                                .unwrap_or_default() =>
                        {
                            *player.get_origin(&mut v)
                        }
                        _ => Vector3::ZERO,
                    };

                    let diff = target - *player.get_origin(&mut v);
                    let angle = diff.y.atan2(diff.x);

                    player.angles.y = angle;

                    // let m = (diff.x * diff.x + diff.y * diff.y + diff.z * diff.z).sqrt();
                    // let mut normlized = diff;

                    // normlized.x /= m;
                    // normlized.x /= m;
                    // normlized.x /= m;

                    [
                        // CUserCmd::new_basic_move(Vector3::ZERO, Action::Duck as u32, &helper),
                        CUserCmd::new_basic_move(
                            Vector3::new(1., 0., 0.),
                            Action::Forward as u32,
                            &helper,
                        ),
                        CUserCmd::new_basic_move(Vector3::ZERO, Action::Walk as u32, &helper),
                        if player.angles.y - angle > 0. {
                            CUserCmd::new_view_shift(Vector3::new(0., -1., 0.), 0, &helper)
                        } else {
                            CUserCmd::new_view_shift(Vector3::new(0., 1., 0.), 0, &helper)
                        },
                    ]
                    .to_vec()
                }
                3 => [CUserCmd::new_basic_move(
                    Vector3::ZERO,
                    Action::Attack as u32,
                    &helper,
                )]
                .to_vec(),
                4 => [CUserCmd::new_basic_move(
                    Vector3::new(-1., -0., 0.),
                    Action::Back as u32 | Action::Walk as u32,
                    &helper,
                )]
                .to_vec(),
                5 => [
                    CUserCmd::new_basic_move(
                        Vector3::new(1., 0., 0.),
                        Action::Forward as u32,
                        &helper,
                    ),
                    CUserCmd::new_basic_move(
                        Vector3::new(0., 0., 0.),
                        Action::Walk as u32,
                        &helper,
                    ),
                ]
                .to_vec(),
                _ => {
                    run_null_command(player);
                    continue;
                }
            };

            **player.rank += 1; // using this for command number

            let amount = cmds.len();

            add_user_cmd_to_player(
                player,
                cmds.as_mut_ptr(),
                amount as u32,
                amount,
                amount as u32,
                0,
            );

            run_null_command(player) // doesn't really work?
        }
    }
}
pub struct CUserCmdHelper<'a> {
    globals: &'a CGlobalVars,
    angles: Vector3,
    cmd_num: u32,
    sv_funcs: &'a ServerFunctions,
}

impl<'a> CUserCmdHelper<'a> {
    pub fn new(
        globals: &'a CGlobalVars,
        angles: Vector3,
        cmd_num: u32,
        sv_funcs: &'a ServerFunctions,
    ) -> CUserCmdHelper<'a> {
        Self {
            globals,
            angles,
            cmd_num,
            sv_funcs,
        }
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

    pub fn new_view_shift(shift: Vector3, buttons: u32, helper: &CUserCmdHelper) -> Self {
        unsafe {
            let new_angles = helper.angles + shift;
            CUserCmd {
                tick_count: **helper.globals.tick_count,
                frame_time: **helper.globals.absolute_frame_time,
                command_time: **helper.globals.cur_time,
                command_number: helper.cmd_num,
                world_view_angles: new_angles,
                attackangles: new_angles,
                buttons,
                tick_something: **helper.globals.tick_count as i32,
                dword90: **helper.globals.tick_count + 4,
                ..CUserCmd::init_default(helper.sv_funcs)
            }
        }
    }
}
