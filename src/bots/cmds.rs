use itertools::Itertools;
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

use super::{BotData, BOT_DATA_MAP, SIMULATE_TYPE_CONVAR};

const GROUND_OFFSET: Vector3 = Vector3::new(0., 0., 20.);
const BOT_VISON_RANGE: f32 = 3000.;
const BOT_PATH_NODE_RANGE: f32 = 50.;
const BOT_PATH_RECAL_RANGE: f32 = 600.;

static mut LAST_CMD: Option<CUserCmd> = None;

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
                camera_angles: helper.angles,
                tick_something: **helper.globals.tick_count as i32,
                dword90: **helper.globals.tick_count + 4,
                ..CUserCmd::init_default(helper.sv_funcs)
            }
        }
    }
}

pub fn replace_cmd() -> Option<&'static CUserCmd> {
    unsafe { LAST_CMD.as_ref() }
}

pub fn run_bots_cmds(_paused: bool) {
    let sim_type = SIMULATE_TYPE_CONVAR.wait().get_value_i32();
    let server_functions = SERVER_FUNCTIONS.wait();
    let engine_functions = ENGINE_FUNCTIONS.wait();
    let player_by_index = server_functions.get_player_by_index;
    let run_null_command = server_functions.run_null_command;
    // let player_process_usercmds = server_functions.player_process_usercmds ;
    let set_base_time = server_functions.set_base_time;
    let player_run_command = server_functions.player_run_command;
    let move_helper = server_functions.get_move_helper;
    let globals =
        unsafe { engine_functions.globals.as_mut() }.expect("globals were null for some reason");

    let helper = CUserCmdHelper::new(
        globals,
        Vector3::ZERO,
        0,
        server_functions,
        engine_functions,
    );

    let mut bot_tasks = BOT_DATA_MAP
        .get(unsafe { EngineToken::new_unchecked() })
        .borrow_mut();

    for (mut cmd, player) in unsafe {
        iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
            .enumerate()
            .filter(|(_, client)| **client.signon == SignonState::FULL)
            .filter(|(_, client)| **client.fake_player)
            .filter_map(|(i, client)| {
                let bot_player = player_by_index((i + 1) as i32).as_mut()?;
                let edict = **client.edict as usize;

                let data = bot_tasks.as_mut().get_mut(edict)?;
                data.edict = edict as u16;
                Some((
                    get_cmd(bot_player, &helper, data.sim_type.unwrap_or(sim_type), data)?,
                    player_by_index((i + 1) as i32).as_mut()?,
                ))
            }) // can collect here to stop the globals from complaning about mutability
    } {
        cmd.frame_time = unsafe { globals.tick_interval.copy_inner() };
        unsafe {
            // add_user_cmd_to_player(
            //     player,
            //     &cmd,
            //     1, // was amount
            //     1, // was amount
            //     0, // was amount as u32, seams like it was causing the dropped packets spam but also it was stoping the bots from going faster?
            //     paused as i8,
            // );

            // LAST_CMD = Some(cmd);

            // bots don't trigger triggers for some reason this way

            // m_pPhysicsController may be behind the crashes in titans

            // checks for m_animActive
            // looks like it still did nothing
            if !*(player as *const _ as *const bool).offset(0xc88)
                || cmd.buttons & Action::WeaponDiscard as u32 == 0
            {
                let frametime = **globals.frametime;
                let cur_time = **globals.cur_time;

                *player.cplayer_state_fixangle.get_inner_mut() = 0;
                set_base_time(player, cur_time);

                let move_helper = move_helper()
                    .cast_mut()
                    .as_mut()
                    .expect("move_helper should not be null");

                move_helper.host = player;

                player_run_command(player, &mut cmd, move_helper);
                *player.latest_command_run.get_inner_mut() = cmd.command_number;
                // (server_functions.set_last_cmd)(
                //     (player as *const _ as *const CUserCmd)
                //         .offset(0x20a0)
                //         .cast_mut(),
                //     &mut cmd,
                // );

                move_helper.host = std::ptr::null_mut();
                #[allow(invalid_reference_casting)] // tmp or not XD
                {
                    *((globals.frametime.get_inner() as *const f32).cast_mut()) = frametime;
                    *((globals.cur_time.get_inner() as *const f32).cast_mut()) = cur_time;
                }

                (server_functions.simulate_player)(player);
            } else {
                run_null_command(player);
            }
            // *player.angles.get_inner_mut() = cmd.world_view_angles // this is not really great -> bad aim
        }
        unsafe { LAST_CMD = None }
    }
}

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

fn look_at(origin: Vector3, target: Vector3) -> Vector3 {
    let diff = target - origin;
    let angley = diff.y.atan2(diff.x).to_degrees();
    let anglex = diff
        .z
        .atan2((diff.x.powi(2) + diff.y.powi(2)).sqrt())
        .to_degrees();

    Vector3::new(-anglex, angley, 0.)
}

fn path_to_target(
    cmd: &mut CUserCmd,
    local_data: &mut BotData,
    origin: Vector3,
    target_pos: Vector3,
    should_recalcute_path: bool,
    helper: &CUserCmdHelper,
) -> bool {
    let dt_funcs = RECAST_DETOUR.wait();
    let debug = ENGINE_INTERFACES.wait().debug_overlay;
    let Some(nav) = local_data.nav_query.as_mut() else {
        log::warn!("null nav");
        return false;
    };

    if distance(target_pos, origin) <= BOT_PATH_NODE_RANGE + 20. {
        return false;
    }

    _ = nav
        .path_points
        .last()
        .map(|point| unsafe { debug.AddLineOverlay(&origin, point, 0, 255, 0, true, 0.1) });
    nav.path_points
        .iter()
        .cloned()
        .tuple_windows()
        .for_each(|(p1, p2)| unsafe { debug.AddLineOverlay(&p1, &p2, 0, 255, 0, true, 0.5) });
    _ = nav
        .path_points
        .last()
        .map(|point| unsafe { debug.AddLineOverlay(point, &target_pos, 0, 255, 0, true, 0.1) });

    if nav
        .path_points
        .first()
        .map(|point| distance(*point, target_pos) > BOT_PATH_RECAL_RANGE)
        .map(|should_recalculate| should_recalculate || should_recalcute_path)
        .unwrap_or(true)
    {
        local_data.last_time_node_reached = unsafe { helper.globals.cur_time.copy_inner() };
        local_data.next_target_pos = origin;

        // this might be the reason of the sudden aim shift or not really idk
        if local_data.last_bad_path + 1. >= unsafe { helper.globals.cur_time.copy_inner() } {
            try_avoid_obstacle(cmd, helper);

            return false;
        }

        if let Err(err) = nav.new_path(origin, target_pos, dt_funcs) {
            log::warn!("navigation pathing failed stuck somewhere probably! {err}");
            try_avoid_obstacle(cmd, helper);

            local_data.last_bad_path = unsafe { helper.globals.cur_time.copy_inner() };

            return false;
        }
    }

    if nav
        .path_points
        .first()
        .cloned()
        .map(|point| distance(point, target_pos) > BOT_PATH_RECAL_RANGE)
        .unwrap_or(true)
    {
        try_avoid_obstacle(cmd, helper);
        cmd.world_view_angles.y = look_at(origin, target_pos).y;

        return true;
    }

    if distance(local_data.next_target_pos, origin) <= BOT_PATH_NODE_RANGE {
        local_data.last_time_node_reached = unsafe { helper.globals.cur_time.copy_inner() };
        local_data.next_target_pos = nav
            .next_point()
            .expect("should always have enough points here");
    }

    cmd.world_view_angles.y = look_at(origin, local_data.next_target_pos).y;
    cmd.move_.x = 1.0;
    cmd.buttons |= Action::Forward as u32 | Action::Speed as u32;

    if is_timedout(local_data.last_time_node_reached, helper, 5.) {
        try_avoid_obstacle(cmd, helper);
    }

    true
}

fn is_timedout(last_time: f32, helper: &CUserCmdHelper<'_>, time_elasped: f32) -> bool {
    last_time + time_elasped <= unsafe { helper.globals.cur_time.copy_inner() }
}

unsafe fn find_player_in_view<'a>(
    pos: Vector3,
    view: Option<Vector3>,
    team: i32,
    helper: &'a CUserCmdHelper,
) -> Option<(&'a mut CPlayer, bool)> {
    const BOT_VIEW: f32 = 270_f32;

    let mut v = Vector3::ZERO;

    if let Some(target) = unsafe {
        let mut possible_targets = enemy_player_iterator(team, helper)
            .map(|player| (*player.get_origin(&mut v), player))
            .filter(|(origin, _)| {
                view.map(|view| dot(normalize(*origin - pos), view) > BOT_VIEW.to_radians().cos())
                    .unwrap_or(true)
            })
            .map(|(target, player)| (target, player, distance(target, pos)))
            .filter(|(_, _, dis)| *dis <= BOT_VISON_RANGE)
            .collect::<Vec<(Vector3, &mut CPlayer, f32)>>();
        possible_targets.sort_by(|(_, _, dis1), (_, _, dis2)| dis1.total_cmp(dis2));

        possible_targets
            .into_iter()
            .find_map(|(target, player, _)| {
                Some(view_rate(helper, pos, target, player, false)).and_then(|(fraction, ent)| {
                    (fraction == 1.0 || ent as usize == player as *const CPlayer as usize)
                        .then(|| view_rate(helper, pos, target, player, true))
                        .and_then(|(fraction, ent)| {
                            (fraction == 1.0 || ent as usize == player as *const CPlayer as usize)
                                .then_some(player)
                        })
                })
            })
    } {
        return Some((target, true));
    }

    None
}

fn farthest_player<'a>(
    pos: Vector3,
    team: i32,
    helper: &'a CUserCmdHelper,
) -> Option<&'a mut CPlayer> {
    distance_iterator(&pos, enemy_player_iterator(team, helper))
        .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
        .map(|(_, player)| player)
}

fn closest_player<'a>(
    pos: Vector3,
    team: i32,
    helper: &'a CUserCmdHelper,
) -> Option<&'a mut CPlayer> {
    distance_iterator(&pos, enemy_player_iterator(team, helper))
        .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
        .map(|(_, player)| player)
}

fn enemy_player_iterator<'b, 'a: 'b>(
    team: i32,
    helper: &'a CUserCmdHelper,
) -> impl Iterator<Item = &'a mut CPlayer> + 'b {
    (0..32)
        .filter_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_mut() })
        .filter(move |player| unsafe { **player.team != team && **player.team != 0 })
        .filter(|player| unsafe { (helper.sv_funcs.is_alive)(*player) != 0 })
}

fn enemy_titan_iterator<'b, 'a: 'b>(
    helper: &'b CUserCmdHelper<'_>,
    team: i32,
) -> impl Iterator<Item = &'a mut CPlayer> + 'b {
    (0..32)
        .filter_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_mut() })
        .filter(move |player| unsafe { **player.team != team && **player.team != 0 })
        .filter_map(|player| {
            unsafe {
                (helper.sv_funcs.get_pet_titan)(player)
                    .cast::<CPlayer>()
                    .cast_mut()
                    .as_mut()
                    .and_then(|titan| (helper.sv_funcs.is_alive)(titan).eq(&1).then_some(titan))
            } // probably safe since the functions should be the same in the vtale, right?
        })
}

fn player_iterator<'b, 'a: 'b>(
    predicate: &'b impl Fn(&CPlayer) -> bool,
    helper: &'a CUserCmdHelper,
) -> impl Iterator<Item = &'a mut CPlayer> + 'b {
    (0..32)
        .filter_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_mut() })
        .filter(|player| predicate(player))
        .filter(|player| unsafe { (helper.sv_funcs.is_alive)(*player) != 0 })
}

fn titan_iterator<'b, 'a: 'b>(
    predicate: &'b impl Fn(&CPlayer) -> bool,
    helper: &'a CUserCmdHelper,
) -> impl Iterator<Item = &'a mut CPlayer> + 'b {
    player_iterator(predicate, helper).filter_map(|player| {
        unsafe {
            (helper.sv_funcs.get_pet_titan)(player)
                .cast::<CPlayer>()
                .cast_mut()
                .as_mut()
        } // probably safe since the functions should be the same in the vtale, right?
    })
}

fn distance_iterator<'b, 'a: 'b>(
    pos: &'b Vector3,
    players: impl Iterator<Item = &'a mut CPlayer> + 'b,
) -> impl Iterator<Item = (i64, &'a mut CPlayer)> + 'b {
    static mut V: Vector3 = Vector3::ZERO;
    players
        .map(|player| {
            (
                unsafe { *player.get_origin(std::ptr::addr_of_mut!(V)) },
                player,
            )
        })
        .map(|(target, player)| (distance(*pos, target) as i64, player))
}

#[allow(unused)]
unsafe fn view_rate(
    helper: &CUserCmdHelper,
    v1: Vector3,
    v2: Vector3,
    player: *mut CPlayer,
    corretness: bool,
) -> (f32, *const CBaseEntity) {
    const TRACE_MASK_SHOT: i32 = 1178615859;
    const TRACE_MASK_SOLID_BRUSHONLY: i32 = 16907;
    const TRACE_COLLISION_GROUP_BLOCK_WEAPONS: i32 = 0x12; // 18

    // should maybe revist the consturction of ray
    let mut result: MaybeUninit<TraceResults> = MaybeUninit::zeroed();
    let mut ray = Ray {
        start: VectorAligned { vec: v1, w: 0. },
        delta: VectorAligned {
            vec: v2 - v1 + GROUND_OFFSET,
            w: 0.,
        },
        offset: VectorAligned {
            vec: Vector3::new(0., 0., 0.),
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
    };

    if corretness {
        let filter: *const CTraceFilterSimple = &CTraceFilterSimple {
            vtable: helper.sv_funcs.simple_filter_vtable,
            unk: 0,
            pass_ent: player.cast(),
            should_hit_func: std::ptr::null(),
            collision_group: TRACE_COLLISION_GROUP_BLOCK_WEAPONS,
        };

        // could use this to get 100% result and trace ray for a aproximation of failure
        (helper.engine_funcs.trace_ray_filter)(
            (*helper.sv_funcs.ctraceengine) as *const libc::c_void,
            &mut ray,
            TRACE_MASK_SHOT as u32,
            filter.cast(),
            result.as_mut_ptr(),
        );
    } else {
        (helper.engine_funcs.trace_ray)(
            (*helper.sv_funcs.ctraceengine) as *const libc::c_void,
            &mut ray,
            TRACE_MASK_SHOT as u32,
            result.as_mut_ptr(),
        );
    }
    let result = result.assume_init();

    if !result.start_solid && result.fraction_left_solid == 0.0 {
        (result.fraction, result.hit_ent)
    } else {
        (0.0, result.hit_ent)
    }
}

fn try_avoid_obstacle(cmd: &mut CUserCmd, helper: &CUserCmdHelper) {
    cmd.move_ = Vector3::new(
        1.,
        if unsafe { helper.globals.frame_count.copy_inner() } / 100 % 2 == 0 {
            -1.
        } else {
            1.
        },
        0.,
    );
    cmd.buttons |= Action::Forward as u32
        | Action::Walk as u32
        | (unsafe { helper.globals.frame_count.copy_inner() } / 10 % 4 == 0)
            .then_some(Action::Jump as u32)
            .unwrap_or_default();
}

fn distance(pos: Vector3, target: Vector3) -> f32 {
    ((pos.x - target.x).powi(2) + (pos.y - target.y).powi(2)).sqrt()
}

fn dot(vec: Vector3, other_vec: Vector3) -> f32 {
    (vec.x * other_vec.x) + (vec.y * other_vec.y) + (vec.z * other_vec.z)
}

fn normalize(vec: Vector3) -> Vector3 {
    let length_recip = dot(vec, vec).sqrt().recip();
    Vector3::new(
        vec.x * length_recip,
        vec.y * length_recip,
        vec.z * length_recip,
    )
}
