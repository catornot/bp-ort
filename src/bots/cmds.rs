use rrplug::{
    bindings::class_types::{client::SignonState, cplayer::CPlayer},
    high::vector::Vector3,
    to_c_string,
};
use std::mem::MaybeUninit;

use crate::{
    bindings::{
        Action, CGlobalVars, CUserCmd, EngineFunctions, ServerFunctions, TraceResults,
        ENGINE_FUNCTIONS, SERVER_FUNCTIONS,
    },
    utils::{client_command, iterate_c_array_sized},
};

use super::{BotData, BotWeaponState, SIMULATE_TYPE_CONVAR, TASK_MAP};

// #[repr(i64)]
// enum weapon_c

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
                    get_cmd(p, &helper, data.sim_type.unwrap_or_else(|| sim_type), data)?,
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
                    to_c_string!(const "CC_RespawnPlayer Pilot\0").as_ptr(),
                )
            };

            CUserCmd::init_default(helper.sv_funcs)
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
                find_closest_player(origin, team, &helper)
                    .map(|player| (*player.get_origin(&mut v), player))
            };

            let mut cmd = CUserCmd::new_basic_move(
                Vector3::ZERO,
                Action::Attack as u32 | Action::Zoom as u32,
                &helper,
            );

            if sim_type == 5 && target.is_some() {
                'ifstmt: {
                    cmd.move_ = Vector3::new(1., 0., 0.);
                    cmd.buttons |= Action::Forward as u32 | Action::Walk as u32;

                    let Some((target, _)) = target else {
                        break 'ifstmt;
                    };

                    if (origin.x - target.x).powi(2) * (origin.y - target.y).powi(2) < 81000. // 810000.
                        && (origin.z - target.z).abs() < 50.
                    {
                        cmd.buttons |= Action::Melee as u32;
                    };
                }
            }

            if let Some((target, target_player)) = target {
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
                    cmd.buttons |= match local_data.counter {
                        0 => Action::OffHand0 as u32,
                        1 => Action::OffHand1 as u32,
                        2 => Action::OffHand2 as u32,
                        3 => Action::OffHand3 as u32,
                        _ => {
                            local_data.counter = 0;
                            Action::OffHand4 as u32
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
            client_command(local_data.edict, to_c_string!(const "say test\0").as_ptr());

            CUserCmd::new_basic_move(
                Vector3::new(1., 0., 0.),
                Action::Forward as u32 | Action::Walk as u32,
                &helper,
            )
        },
        7 => CUserCmd::new_basic_move(
            Vector3::new(1., 0., 0.),
            Action::Forward as u32 | Action::Walk as u32,
            &helper,
        ),
        _ => CUserCmd::init_default(helper.sv_funcs),
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
) -> Option<&'a mut CPlayer> {
    let mut v = Vector3::ZERO;
    if let Some(target) = unsafe {
        (0..32)
            .filter_map(|i| (helper.sv_funcs.get_player_by_index)(i + 1).as_mut())
            .filter(|player| **player.team != team)
            .filter(|player| (helper.sv_funcs.is_alive)(*player) != 0)
            .map(|player| (*player.get_origin(&mut v), player))
            .map(|(target, player)| (view_rate(helper, pos, target, player), player))
            .find(|(dist, _)| *dist >= 1000)
            .map(|(_, player)| player)
    } {
        return Some(target);
    }

    let mut targets = unsafe {
        (0..32)
            .filter_map(|i| (helper.sv_funcs.get_player_by_index)(i + 1).as_mut())
            .filter(|player| **player.team != team)
            .filter(|player| (helper.sv_funcs.is_alive)(*player) != 0)
            .map(|player| (*player.get_origin(&mut v), player))
            // .map(|(target, player)| (view_rate(helper, pos, target, player), player))
            .map(|(target, player)| (distance_rate(pos, target), player))
            // .filter(|(target, _)| view_rate(helper, pos, *target) == 1000)
            .collect::<Vec<(i64, &mut CPlayer)>>()
    };

    targets.sort_by(|(dis1, _), (dis2, _)| dis1.cmp(dis2));
    // targets.into_iter().last().map(|(_, player)| player) // for view_rate
    targets.into_iter().next().map(|(_, player)| player) // for distance_rate
}

#[allow(unused)]
unsafe fn view_rate(
    helper: &CUserCmdHelper,
    v1: Vector3,
    v2: Vector3,
    player: *mut CPlayer,
) -> i64 {
    const POS_OFFSET: Vector3 = Vector3::new(0., 0., 50.);

    let (v1, v2) = (v1 + POS_OFFSET, v2 + POS_OFFSET);

    // let id = ((player.offset(0x30) as usize) & 0xffff);
    // let id = (helper.sv_funcs.base.offset(0xb6ab58) as usize + id * 30) as i8 * -1;

    let mut result: MaybeUninit<TraceResults> = MaybeUninit::zeroed();
    (helper.sv_funcs.trace_line_simple)(&v2, &v1, -1, 0, 0, 0, 0, result.as_mut_ptr());
    let result = result.assume_init();

    (result.fraction * 1000.) as i64
}

#[allow(unused)]
fn distance_rate(pos: Vector3, target: Vector3) -> i64 {
    ((pos.x - target.x).powi(2) as f64 * (pos.y - target.y).powi(2) as f64) as i64
}
