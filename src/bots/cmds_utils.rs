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

use super::{cmds_helper::CUserCmdHelper, BotData};

pub fn look_at(origin: Vector3, target: Vector3) -> Vector3 {
    let diff = target - origin;
    let angley = diff.y.atan2(diff.x).to_degrees();
    let anglex = diff
        .z
        .atan2((diff.x.powi(2) + diff.y.powi(2)).sqrt())
        .to_degrees();

    Vector3::new(-anglex, angley, 0.)
}

pub fn path_to_target(
    cmd: &mut CUserCmd,
    local_data: &mut BotData,
    origin: Vector3,
    target_pos: Vector3,
    should_recalcute_path: bool,
    helper: &CUserCmdHelper,
) -> bool {
    let dt_funcs = RECAST_DETOUR.wait();
    #[cfg(not(feature = "release"))]
    let debug = ENGINE_INTERFACES.wait().debug_overlay;
    let Some(nav) = local_data.nav_query.as_mut() else {
        log::warn!("null nav");
        return false;
    };

    if distance(target_pos, origin) <= BOT_PATH_NODE_RANGE + 20. {
        return false;
    }

    #[cfg(not(feature = "release"))]
    {
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
    }

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

pub fn is_timedout(last_time: f32, helper: &CUserCmdHelper<'_>, time_elasped: f32) -> bool {
    last_time + time_elasped <= unsafe { helper.globals.cur_time.copy_inner() }
}

pub unsafe fn find_player_in_view<'a>(
    pos: Vector3,
    view: Option<Vector3>,
    team: i32,
    helper: &'a CUserCmdHelper,
) -> Option<(&'a mut CPlayer, bool)> {
    const BOT_VIEW: f32 = 270_f32;

    let mut v = Vector3::ZERO;

    if let Some(target) = unsafe {
        let mut possible_targets = enemy_player_iterator(team, helper)
            .chain(enemy_titan_iterator(helper, team))
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

pub fn farthest_player<'a>(
    pos: Vector3,
    team: i32,
    helper: &'a CUserCmdHelper,
) -> Option<&'a mut CPlayer> {
    distance_iterator(&pos, enemy_player_iterator(team, helper))
        .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
        .map(|(_, player)| player)
}

pub fn closest_player<'a>(
    pos: Vector3,
    team: i32,
    helper: &'a CUserCmdHelper,
) -> Option<&'a mut CPlayer> {
    distance_iterator(&pos, enemy_player_iterator(team, helper))
        .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
        .map(|(_, player)| player)
}

pub fn enemy_player_iterator<'b, 'a: 'b>(
    team: i32,
    helper: &'a CUserCmdHelper,
) -> impl Iterator<Item = &'a mut CPlayer> + 'b {
    (0..32)
        .filter_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_mut() })
        .filter(move |player| unsafe { **player.team != team && **player.team != 0 })
        .filter(|player| unsafe { (helper.sv_funcs.is_alive)(*player) != 0 })
}

pub fn enemy_titan_iterator<'b, 'a: 'b>(
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

pub fn player_iterator<'b, 'a: 'b>(
    predicate: &'b impl Fn(&CPlayer) -> bool,
    helper: &'a CUserCmdHelper,
) -> impl Iterator<Item = &'a mut CPlayer> + 'b {
    (0..32)
        .filter_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_mut() })
        .filter(|player| predicate(player))
        .filter(|player| unsafe { (helper.sv_funcs.is_alive)(*player) != 0 })
}

pub fn titan_iterator<'b, 'a: 'b>(
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

pub fn distance_iterator<'b, 'a: 'b>(
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
pub unsafe fn view_rate(
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

pub fn try_avoid_obstacle(cmd: &mut CUserCmd, helper: &CUserCmdHelper) {
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

pub fn distance(pos: Vector3, target: Vector3) -> f32 {
    ((pos.x - target.x).powi(2) + (pos.y - target.y).powi(2)).sqrt()
}

pub fn dot(vec: Vector3, other_vec: Vector3) -> f32 {
    (vec.x * other_vec.x) + (vec.y * other_vec.y) + (vec.z * other_vec.z)
}

pub fn normalize(vec: Vector3) -> Vector3 {
    let length_recip = dot(vec, vec).sqrt().recip();
    Vector3::new(
        vec.x * length_recip,
        vec.y * length_recip,
        vec.z * length_recip,
    )
}