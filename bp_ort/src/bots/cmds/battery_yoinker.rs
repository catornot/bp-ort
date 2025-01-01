use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};

use crate::{
    bindings::{Action, CUserCmd},
    bots::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData},
    utils::get_net_var,
};

pub(crate) fn battery_yoinker(
    helper: &CUserCmdHelper,
    player: &mut CPlayer,
    local_data: &mut BotData,
) -> CUserCmd {
    let mut v = Vector3::ZERO;
    let v = &mut v;
    let mut cmd =
        CUserCmd::new_basic_move(Vector3::new(1., 0., 0.), Action::Forward as u32, helper);
    let origin = unsafe { *player.get_origin(v) };
    let team = unsafe { **player.team };
    local_data.counter = local_data.counter.wrapping_add(1);

    if unsafe { player.titan_soul_being_rodeoed.copy_inner() } != -1 {
        log::info!(
            "{} {}",
            local_data.last_shot,
            is_timedout(local_data.last_shot, helper, 20.)
        );

        if is_timedout(local_data.last_shot, helper, 10.) && local_data.counter / 10 % 4 == 0 {
            cmd.buttons |= Action::Jump as u32 | Action::WeaponDiscard as u32;
        }
        return cmd;
    } else {
        local_data.last_shot = unsafe { helper.globals.cur_time.copy_inner() };
    }

    let is_team = move |player: &CPlayer| -> bool { unsafe { **player.team == team } };
    let maybe_rodeo_target = get_net_var(player, c"batteryCount", 191, helper.sv_funcs)
        .and_then(|value| value.eq(&0).then_some(()))
        .and_then(|_| {
            distance_iterator(
                &origin,
                enemy_player_iterator(team, helper)
                    .chain(enemy_titan_iterator(helper, team))
                    .filter(|ent| unsafe { (helper.sv_funcs.is_titan)(*ent) }),
            )
            .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
            .map(|(_, player)| unsafe { *player.get_origin(v) })
        })
        .or_else(|| {
            distance_iterator(
                &origin,
                player_iterator(&is_team, helper)
                    .chain(titan_iterator(&is_team, helper))
                    .filter(|ent| unsafe { (helper.sv_funcs.is_titan)(*ent) }),
            )
            .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
            .map(|(_, player)| unsafe { *player.get_origin(v) })
        });

    if let Some(rodeo_target) = maybe_rodeo_target {
        if distance(origin, rodeo_target) > 100. {
            path_to_target(&mut cmd, local_data, origin, rodeo_target, false, helper);
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
