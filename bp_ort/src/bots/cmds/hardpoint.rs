use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};
use std::cell::UnsafeCell;

use crate::{
    bindings::CUserCmd,
    bots::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData},
    utils::get_ents_by_class_name,
};

static HARDPOINT_DATA: EngineGlobal<UnsafeCell<HardPointData>> =
    EngineGlobal::new(UnsafeCell::new(HardPointData {
        last_checked: -1,
        hardpoints: Vec::new(),
    }));

struct HardPointData {
    last_checked: i32,
    hardpoints: Vec<Vector3>,
}

pub fn basic_cap_holding(
    player: &mut CPlayer,
    helper: &CUserCmdHelper<'_>,
    local_data: &mut BotData,
    origin: Vector3,
    cmd: &mut CUserCmd,
    target: &Option<((Vector3, &mut CPlayer), bool)>,
) {
    let mut v = Vector3::ZERO;
    let team = unsafe { player.team.copy_inner() };
    let predicate = |other: &CPlayer| unsafe { other.team.copy_inner() } == team && !std::ptr::eq(other, player);
    let allied_player_count = player_iterator(&predicate, helper).count();
    let prefered_hardpoint = get_hardpoints(helper)
        .map(|hardpoint| {
            (
                player_iterator(&predicate, helper)
                    .map(|player| unsafe { *player.get_origin(&mut v) })
                    .filter(|pos| distance(*pos, hardpoint) < 500.)
                    .count(),
                distance(hardpoint, origin),
                hardpoint,
            )
        })
        .filter(move |(player_around, _, _)| *player_around <= allied_player_count / 3)
        .reduce(|first, second| if first.1 <= second.1 { first } else { second })
        .map(|(_, _, pos)| pos);

    let (new_target_pos, should_recaculate) = if let Some(hardpoint) = prefered_hardpoint {
        if distance3(hardpoint, origin) <= 200. {
            (
                local_data
                    .nav_query
                    .as_mut()
                    .and_then(|nav| nav.random_point_around(hardpoint, 200.))
                    .unwrap_or_else(|| hardpoint + Vector3::new(0., 0., 50.)),
                None,
            )
        } else {
            local_data.approach_range = Some(100.);
            (hardpoint, None)
        }
    } else if let Some(((target_pos, target), _)) = target.as_ref() {
        (
            *target_pos,
            Some(local_data.last_target_index != unsafe { target.player_index.copy_inner() }),
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
