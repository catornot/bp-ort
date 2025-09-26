use rrplug::{
    bindings::class_types::{cbaseentity::CBaseEntity, cplayer::CPlayer},
    prelude::*,
};
use shared::utils::nudge_type;

use crate::{
    bindings::{Action, CUserCmd},
    bots::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData},
};

pub(crate) fn slide_hopper(
    helper: &CUserCmdHelper,
    player: &mut CPlayer,
    local_data: &mut BotData,
) -> CUserCmd {
    let mut v = Vector3::ZERO;
    let v = &mut v;
    let mut cmd =
        CUserCmd::new_basic_move(Vector3::new(1., 0., 0.), Action::Forward as u32, helper);
    let origin = unsafe { *player.get_origin(v) };

    let target = closest_player(origin, player.m_iTeamNum, helper)
        .map(|target| unsafe { *target.get_origin(v) });

    if let Some(target) = target {
        let length = get_velocity_length(helper, player, v);
        let touching_ground =
            unsafe { (helper.sv_funcs.is_on_ground)(nudge_type::<&CBaseEntity>(player)) } != 0;

        if length <= 50. && local_data.has_started_to_slide_hop {
            local_data.has_started_to_slide_hop = false;
            local_data.should_recaculate_path = true;
        } else if !local_data.has_started_to_slide_hop && length <= 50. && touching_ground {
            path_to_target(&mut cmd, local_data, origin, target, false, helper);
            cmd.buttons |= Action::Jump as u32 | Action::Duck as u32;
        } else if !touching_ground {
            path_to_target(&mut cmd, local_data, origin, target, false, helper);
            cmd.buttons |= Action::Duck as u32;
        }
    } else {
        local_data.has_started_to_slide_hop = false;
        cmd.move_ = Vector3::ZERO;
    }

    cmd
}
