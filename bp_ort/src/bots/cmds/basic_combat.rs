use high::squirrel::call_sq_function;
use mid::squirrel::SQVM_SERVER;
use rand::{thread_rng, Rng};
use rrplug::{
    bindings::class_types::cplayer::CPlayer, high::UnsafeHandle, prelude::EngineToken, prelude::*,
};

use crate::{
    bindings::{Action, CBaseEntity, CUserCmd},
    bots::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData},
    utils::get_net_var,
};

pub(crate) fn basic_combat(
    player: &mut CPlayer,
    helper: &CUserCmdHelper,
    sim_type: i32,
    local_data: &mut BotData,
) -> CUserCmd {
    let mut v = Vector3::ZERO;
    let v = &mut v;
    let origin = unsafe { *player.get_origin(v) };
    let team = unsafe { **player.team };

    let target = unsafe {
        find_player_in_view(
            origin,
            Some(*(helper.sv_funcs.view_angles)(player, v)),
            team,
            helper,
        )
        .map(|(player, should_shoot)| ((*player.get_origin(v), player), should_shoot))
        .or_else(|| {
            distance_iterator(
                &origin,
                enemy_player_iterator(team, helper).chain(enemy_titan_iterator(helper, team)),
            )
            .reduce(|closer, other| if closer.0 < other.0 { other } else { closer })
            .map(|(_, player)| player)
            .map(|player| ((*player.get_origin(v), player), false))
        })
    };

    let mut cmd = CUserCmd::new_basic_move(Vector3::ZERO, 0, helper);

    match (sim_type, &target) {
        (6, target) if target.is_none() || matches!(target, Some((_, false))) => {
            if let Some(pet_titan) = unsafe { (helper.sv_funcs.get_pet_titan)(player).as_ref() } {
                path_to_target(
                    &mut cmd,
                    local_data,
                    origin,
                    unsafe {
                        *(helper.sv_funcs.get_origin)((pet_titan as *const CBaseEntity).cast(), v)
                    },
                    local_data.should_recaculate_path,
                    helper,
                );
            } else if let Some(((target_pos, target), _)) = target {
                if path_to_target(
                    &mut cmd,
                    local_data,
                    origin,
                    *target_pos,
                    local_data.last_target_index != unsafe { target.player_index.copy_inner() }
                        || local_data.should_recaculate_path,
                    helper,
                ) {
                    local_data.last_target_index = unsafe { target.player_index.copy_inner() }
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
                helper,
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
        cmd.buttons |= if should_shoot && is_timedout(local_data.last_shot, helper, 0.8) {
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
                *(helper.sv_funcs.get_origin)((titan as *const CBaseEntity).cast::<CPlayer>(), v)
            };

            let (dis, ent) = unsafe { view_rate(helper, titan_pos, origin, player, true) };
            if dis >= 1.0 || ent == titan as *const CBaseEntity {
                if (origin.x - titan_pos.x).powi(2) * (origin.y - titan_pos.y).powi(2) < 81000.
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

            let angles = {
                let length = { get_velocity_length(helper, target_player, v) };

                if length > 200. {
                    let mut rng = thread_rng();
                    let error_amount = length.sqrt() / 10f32;

                    Vector3 {
                        x: angles.x + error_amount * rng.gen_range(-2..=2) as f32,
                        y: angles.y + error_amount * rng.gen_range(-2..=2) as f32,
                        z: 0.,
                    }
                } else {
                    angles
                }
            };

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
            use crate::bots::TitanClass as TC;
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

    if is_timedout(local_data.next_check, helper, 10f32)
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
