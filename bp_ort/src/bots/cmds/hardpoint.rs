use once_cell::sync::Lazy;
use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};
use std::{cell::UnsafeCell, collections::HashMap};

use crate::{
    bindings::CUserCmd,
    bots::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData},
    utils::get_ents_by_class_name,
};

static HARDPOINT_DATA: EngineGlobal<UnsafeCell<HardPointData>> =
    EngineGlobal::new(UnsafeCell::new(HardPointData {
        last_checked: -1,
        hardpoints: Vec::new(),
        claimed: Lazy::new(HashMap::new),
    }));

struct HardPointData {
    last_checked: i32,
    hardpoints: Vec<Vector3>,
    claimed: Lazy<HashMap<[i32; 2], Option<i32>>>,
}

pub fn reset_hardpoint(bot: Option<&CPlayer>, local_data: Option<&mut BotData>) {
    match (bot, unsafe {
        HARDPOINT_DATA
            .get(EngineToken::new_unchecked())
            .get()
            .as_mut()
    }) {
        (Some(bot), Some(data)) => revoke_claim(bot, data),
        (Option::None, Some(data)) => data.claimed.clear(),
        _ => {}
    }

    if let Some(local_data) = local_data {
        local_data.patrol_target = None;
    }
}

pub fn basic_cap_holding(
    player: &mut CPlayer,
    helper: &CUserCmdHelper<'_>,
    local_data: &mut BotData,
    origin: Vector3,
    cmd: &mut CUserCmd,
    target: &Option<((Vector3, &mut CPlayer), bool)>,
) {
    const APROCHE_DISTANCE: f32 = 300.;

    let mut v = Vector3::ZERO;
    let team = player.m_iTeamNum;
    let predicate = |other: &CPlayer| other.m_iTeamNum == team && !std::ptr::eq(other, player);
    let allied_player_count = player_iterator(&predicate, helper).count();
    let prefered_hardpoint = get_claimed_hardpoint(player).or_else(|| {
        get_hardpoints(helper)
            .map(|hardpoint| {
                (
                    player_iterator(&predicate, helper)
                    .filter(|player| player.m_iTeamNum == team)
                    .map(|player| (unsafe { *player.get_origin(&mut v) }, player.pl.index))
                    .filter(|(pos, index)| {
                        distance(*pos, hardpoint) < APROCHE_DISTANCE + 200.
                            // removed claimed players
                            && get_hardpoint_claim(hardpoint)
                                .filter(|around_index| *around_index == *index).is_none()
                    })
                    .count()
                    // count up the claimed
                    + get_hardpoint_claim(hardpoint)
                        .iter()
                        .count(),
                    distance(hardpoint, origin),
                    hardpoint,
                )
            })
            .filter(move |(player_around, _, _)| *player_around < (allied_player_count / 6).max(1))
            .reduce(|first, second| if first.1 <= second.1 { first } else { second })
            .map(|(_, _, pos)| pos)
    });

    let (new_target_pos, should_recaculate) = if let Some(hardpoint) = prefered_hardpoint {
        if distance3(hardpoint, origin) <= APROCHE_DISTANCE {
            claim_hardpoint(hardpoint, player);

            let patrol_target = local_data.patrol_target.unwrap_or_else(|| {
                local_data
                    .nav_query
                    .as_mut()
                    .and_then(|nav| nav.random_point_around(hardpoint, APROCHE_DISTANCE, None))
                    .unwrap_or_else(|| hardpoint + Vector3::new(0., 50., 50.))
            });
            local_data.patrol_target = Some(patrol_target);

            if distance(patrol_target, origin) <= 50.
                || is_timedout(local_data.last_moved_from_cap, helper, 2.)
            {
                local_data.patrol_target.take();
                local_data.last_moved_from_cap = time(helper);
            }

            (
                local_data
                    .patrol_target
                    .unwrap_or(hardpoint + Vector3::new(0., 50., 50.)),
                Some(local_data.patrol_target.map(|_| false).unwrap_or(true)),
            )
        } else {
            revoke_claim(player, unsafe {
                &mut *HARDPOINT_DATA.get(EngineToken::new_unchecked()).get()
            });
            local_data.approach_range = Some(90.);
            (hardpoint, None)
        }
    } else if let Some(((target_pos, target), _)) = target.as_ref() {
        (
            *target_pos,
            Some(local_data.last_target_index != target.pl.index),
        )
    } else {
        local_data.approach_range = Some(300.);
        (get_hardpoints(helper).next().unwrap_or_default(), None)
    };

    _ = path_to_target(
        cmd,
        local_data,
        origin,
        new_target_pos,
        should_recaculate.unwrap_or_else(|| local_data.target_pos != new_target_pos)
            || local_data.should_recaculate_path,
        helper,
    );

    // not the actual use but it's okay
    local_data.target_pos = new_target_pos;
    local_data.should_recaculate_path = false;
    local_data.approach_range = None;
}

fn get_hardpoints(helper: &CUserCmdHelper) -> impl Iterator<Item = Vector3> {
    let token = try_refresh_hardpoint(helper);

    unsafe { &*HARDPOINT_DATA.get(token).get() }
        .hardpoints
        .iter()
        .copied()
}

fn get_hardpoint_claim(hardpoint: Vector3) -> Option<i32> {
    unsafe { &*HARDPOINT_DATA.get(EngineToken::new_unchecked()).get() }
        .claimed
        .get(&[hardpoint.x as i32, hardpoint.y as i32])
        .cloned()
        .flatten()
}

fn get_claimed_hardpoint(bot: &CPlayer) -> Option<Vector3> {
    let data = unsafe { &*HARDPOINT_DATA.get(EngineToken::new_unchecked()).get() };
    data.hardpoints
        .iter()
        .copied()
        .map(|hardpoint| {
            (
                hardpoint,
                data.claimed
                    .get(&[hardpoint.x as i32, hardpoint.y as i32])
                    .cloned()
                    .flatten(),
            )
        })
        .filter(|(_, claim)| *claim == Some(bot.pl.index))
        .map(|(hardpoint, _)| hardpoint)
        .last()
}

fn claim_hardpoint(hardpoint: Vector3, bot: &CPlayer) -> bool {
    let data = unsafe { &mut *HARDPOINT_DATA.get(EngineToken::new_unchecked()).get() };
    if data
        .claimed
        .get_mut(&[hardpoint.x as i32, hardpoint.y as i32])
        .filter(|claim| claim.is_none())
        .map(|claim| claim.replace(bot.pl.index))
        .is_some()
    {
        revoke_claim(bot, data);
        return true;
    }
    false
}

fn revoke_claim(bot: &CPlayer, data: &mut HardPointData) {
    data.claimed
        .values_mut()
        .filter(|hardpoint_player| hardpoint_player.filter(|p| *p == bot.pl.index).is_some())
        .for_each(|hardpoint_player| _ = hardpoint_player.take());
}
fn try_refresh_hardpoint(helper: &CUserCmdHelper) -> EngineToken {
    let token = unsafe { EngineToken::new_unchecked() };
    let data = unsafe { &mut *HARDPOINT_DATA.get(token).get() };
    let mut v = Vector3::ZERO;

    if data.last_checked == helper.globals.frameCount {
        return token;
    }
    data.last_checked = helper.globals.frameCount;

    data.hardpoints.clear();

    data.hardpoints.extend(
        get_ents_by_class_name(c"info_hardpoint", helper.sv_funcs).map(|ent| unsafe {
            *ent.cast::<CPlayer>()
                .as_ref()
                .unwrap_unchecked()
                .get_origin(&mut v)
        }),
    );

    token
}
