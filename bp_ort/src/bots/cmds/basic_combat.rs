#![deny(clippy::unwrap_used, clippy::expect_used)]
use high::squirrel::call_sq_function;
use mid::squirrel::SQVM_SERVER;
use rand::{thread_rng, Rng};
use rrplug::{
    bindings::class_types::{cbaseentity::CBaseEntity, cplayer::CPlayer, cweaponx::CWeaponX},
    high::UnsafeHandle,
    prelude::{EngineToken, *},
};
use shared::utils::nudge_type;
use std::{cell::UnsafeCell, sync::atomic::Ordering};

use crate::{
    bindings::{Action, CUserCmd},
    bots::{cmds_helper::CUserCmdHelper, cmds_utils::*, BotData, AIM_PENALTY_VALUE},
    utils::{get_ents_by_class_name, get_net_var, get_weaponx_name, lookup_ent},
};

use super::BotShared;

/// this cannot be accessed from multiple places so it's safe
static HEADHUNTER_DATA: EngineGlobal<UnsafeCell<HeadHunterData>> =
    EngineGlobal::new(UnsafeCell::new(HeadHunterData {
        last_checked: -1,
        batteries: Vec::new(),
        hardpoints: Vec::new(),
    }));
static CTF_DATA: EngineGlobal<UnsafeCell<CtfData>> = EngineGlobal::new(UnsafeCell::new(CtfData {
    last_checked: -1,
    flags: Vec::new(),
    bases: Vec::new(),
}));

struct HeadHunterData {
    last_checked: i32,
    batteries: Vec<Vector3>,
    hardpoints: Vec<Vector3>,
}

struct CtfData {
    last_checked: i32,
    flags: Vec<(Vector3, i32, u32)>,
    bases: Vec<(Vector3, i32)>,
}

// TODO INFO bots may be crashing "in titans" when in fact they crash when checking a player when they are embarking
pub(crate) fn basic_combat(
    player: &mut CPlayer,
    helper: &CUserCmdHelper,
    sim_type: i32,
    local_data: &mut BotData,
    shared_data: &mut BotShared,
) -> CUserCmd {
    let mut v = Vector3::ZERO;
    let v = &mut v;
    let origin = unsafe { *player.get_origin(v) };
    let team = player.m_iTeamNum;
    let is_titan = unsafe { (helper.sv_funcs.is_titan)(nudge_type::<&CBaseEntity>(player)) };

    let target = unsafe {
        find_player_in_view(
            origin,
            Some(*(helper.sv_funcs.view_angles)(player, v)),
            team,
            helper,
            None,
            None,
            // disabled for now since it doesn't actually help instead it makes them worse
            // Some(player.player_index.copy_inner()),
            // Some(shared.reserved_targets.as_ref()),
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
                    unsafe { *pet_titan.get_origin(v) },
                    local_data.should_recaculate_path,
                    helper,
                );
            } else if let Some(((target_pos, target), _)) = target
                && path_to_target(
                    &mut cmd,
                    local_data,
                    origin,
                    *target_pos,
                    local_data.last_target_index != target.pl.index
                        || local_data.should_recaculate_path,
                    helper,
                )
            {
                local_data.last_target_index = target.pl.index
            }

            local_data.should_recaculate_path = false;
        }
        (7, vision) if vision.is_none() || matches!(vision, Some((_, false))) => {
            // this may be a big issue here
            // this sometimes does need to recaculate the path
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
        (8, target) if target.is_none() || matches!(target, Some((_, false))) => {
            let (new_target_pos, should_recaculate) =
            // check if team members are dead
                if get_net_var(player, c"batteryCount", 191, helper.sv_funcs).unwrap_or(0) > 0{
                    local_data.approach_range = Some(70.);
                    (find_closest_hardpoint(origin, helper), None)
                } else if let Some(battery) = find_closest_battery(origin, helper) {
                    local_data.approach_range = Some(-20.);
                    (battery, None)
                } else if let Some(((target_pos, target), _)) = target {
                    let result = (
                        *target_pos,
                        Some(
                            local_data.last_target_index
                                != target.pl.index ,
                        ),
                    );
                    local_data.last_target_index = target.pl.index ;

                    result
                } else {
                    local_data.approach_range = Some(200.);
                    (find_closest_hardpoint(origin, helper), None)
                };

            _ = path_to_target(
                &mut cmd,
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
        (9, target) if target.is_none() || matches!(target, Some((_, false))) => {
            let (team, player_index) = (player.m_iTeamNum, player.pl.index as u32);
            let our_flag = find_flag_for(team, true, player_index, helper);
            let their_flag = find_flag_for(team, false, player_index, helper);
            let our_base = find_base_for(team, true, helper);
            let _their_base = find_base_for(team, false, helper);

            let is_team = move |player: &CPlayer| -> bool { player.m_iTeamNum == team };
            // mm allocation every frame
            let mut friendly_players = player_iterator(&is_team, helper)
                .map(|friendly| {
                    (
                        unsafe { distance(*friendly.get_origin(v), their_flag.0) },
                        std::ptr::eq(friendly, player),
                    )
                })
                .collect::<Vec<_>>();
            friendly_players.sort_by(|(dis, _), (other_dis, _)| dis.total_cmp(other_dis));

            let (new_target_pos, should_recaculate) = if let Some(pet_titan) =
                unsafe { (helper.sv_funcs.get_pet_titan)(player).as_ref() }
            {
                (unsafe { *pet_titan.get_origin(v) }, Some(false))
            } else if their_flag.1 {
                local_data.approach_range = Some(-20.);
                (our_base, None)
            } else if friendly_players
                .iter()
                .position(|(_, is_self)| *is_self)
                .unwrap_or(usize::MAX)
                < friendly_players.len() / 2
            {
                local_data.approach_range = Some(-20.);
                (their_flag.0, None)
            } else if distance(our_flag.0, our_base) > 30. {
                local_data.approach_range = Some(0.);
                (our_flag.0, None)
            } else if ((time(helper) as u64) / 30) % 2 == 0 {
                (their_flag.0, None)
            } else {
                (our_flag.0, None)
            };

            _ = path_to_target(
                &mut cmd,
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
        (10, target) if target.is_none() || matches!(target, Some((_, false))) => {
            super::hardpoint::basic_cap_holding(
                player, helper, local_data, origin, &mut cmd, target,
            );
        }
        (_, Some((_, _))) => {
            cmd.move_ = Vector3::new(1., 0., 0.);
            cmd.buttons |= Action::Forward as u32 | Action::Walk as u32;

            local_data.should_recaculate_path = true;
        }
        _ => {}
    }

    cmd.buttons |= if matches!(target, Some((_, true))) && !is_titan {
        // pilot abilities TODO: check for cooldown
        match lookup_ent(player.m_inventory.offhandWeapons[1], helper.sv_funcs)
                .and_then(|ent| get_weaponx_name(ent, helper.sv_funcs))
                .unwrap_or_default()
            {
                "mp_weapon_grenade_sonar" => Action::OffHand1 as u32,
                "mp_ability_cloak" => Action::OffHand1 as u32,
                "mp_ability_grapple" => 0,
                "mp_ability_heal" if player.m_iHealth < 50 => Action::OffHand1 as u32,
                "mp_ability_shifter" if player.m_iHealth < 50 => Action::OffHand1 as u32,
                "mp_ability_holopilot" if player.m_iHealth < 50 => Action::OffHand1 as u32,
                _ => 0,
            }
            .saturating_mul((time(helper) as u32 % 25 < 10) as u32)
                // pilot nades TODO: check for cooldown
                | match lookup_ent(player.m_inventory.offhandWeapons[0], helper.sv_funcs)
                    .and_then(|ent| get_weaponx_name(ent, helper.sv_funcs))
                    .unwrap_or_default()
                {
                    "mp_ability_shifter" if player.m_iHealth < 50 => Action::OffHand0 as u32,
                    "mp_weapon_grenade_sonar" => Action::OffHand0 as u32,
                    "mp_weapon_grenade_gravity" => Action::OffHand0 as u32,
                    "mp_weapon_thermite_grenade" => Action::OffHand0 as u32,
                    "mp_weapon_grenade_emp" => Action::OffHand0 as u32,
                    _ => 0,
                }
                .saturating_mul((time(helper) as u32 % 25 < 3) as u32)
    } else {
        0
    };

    if let Some(((target, target_player), should_shoot)) = target {
        if let Some(target) = shared_data
            .reserved_targets
            .get_mut(target_player.pl.index as usize)
        {
            // a last shot target system would be a lot better imo or even prefered target
            *target = (time(helper), player.pl.index as u32);
        }

        let active_weapon = lookup_ent(player.m_inventory.activeWeapon, helper.sv_funcs)
            .and_then::<&CWeaponX, _>(|weapon| weapon.dynamic_cast());

        // 0x2c is the offset to weather the weapon can charge or not
        let (should_charge, charge_weapon) = active_weapon
            .map(|weapon| (weapon, weapon.m_lastChargeFrac != 0.))
            .map::<(&CBaseEntity, bool), _>(|x| (x.0, x.1))
            .map(|(weapon, is_charge)| unsafe {
                let charge = (helper.sv_funcs.get_weapon_charge_fraction)(weapon);
                (
                    is_charge
                        && charge < 1.
                        && !(charge == 0. && helper.globals.frameCount / 2 % 4 != 0),
                    is_charge,
                )
            })
            .unwrap_or_default();

        cmd.buttons |= if should_shoot && is_timedout(local_data.last_shot, helper, 0.8) {
            Action::Zoom as u32
                | if (!charge_weapon && helper.globals.frameCount / 2 % 4 != 0) || should_charge {
                    Action::Attack as u32
                } else {
                    Default::default()
                }
        } else if should_shoot {
            0
        } else {
            local_data.last_shot = helper.globals.curTime;
            0
        };

        let target =
            if let Some(titan) = unsafe { (helper.sv_funcs.get_pet_titan)(player).as_ref() } {
                let titan_pos = unsafe { *titan.get_origin(v) };

                let (dis, ent) = unsafe { view_rate(helper, titan_pos, origin, player, true) };
                if dis >= 1.0 || ent == std::ptr::from_ref(titan) {
                    if (origin.x - titan_pos.x).powi(2) * (origin.y - titan_pos.y).powi(2) < 81000.
                        && (helper.globals.frameCount / 2 % 4 != 0)
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

        let enemy_is_titan =
            unsafe { (helper.sv_funcs.is_titan)(nudge_type::<&mut CBaseEntity>(target_player)) };

        if should_shoot || sim_type == 5 || sim_type == 4 {
            // for some reason this makes the game freeze later in the frame????
            // if local_data.spread.len() < local_data.spread_offset {
            //     generate_spread(player.m_vecAbsVelocity, &mut local_data.spread);
            //     local_data.spread_offset = 0;
            // }

            let angles = look_at(origin, target);

            let angles = {
                let length = { get_velocity_length(helper, target_player, v) };

                if length > AIM_PENALTY_VALUE.load(Ordering::Relaxed) as f32 && !enemy_is_titan {
                    let mut rng = thread_rng();
                    let error_amount = (length + 50.).sqrt() / 7f32;

                    Vector3 {
                        x: angles.x + error_amount * rng.gen_range(-2..=2) as f32,
                        y: angles.y + error_amount * rng.gen_range(-2..=2) as f32,
                        z: 0.,
                    }
                } else if sim_type == 5 || sim_type == 4 {
                    angles
                } else {
                    local_data.spread_offset += 1;

                    angles
                        + local_data
                            .spread
                            .get(local_data.spread_offset)
                            .copied()
                            .unwrap_or(Vector3::ZERO)
                }
            };

            const CLAMP: f32 = 10.;

            cmd.world_view_angles.x = angles.x;
            cmd.world_view_angles.y = if angles.y.is_finite() {
                {
                    angles.y.clamp(
                        cmd.world_view_angles.y - CLAMP,
                        cmd.world_view_angles.y + CLAMP,
                    )
                }
            } else {
                angles.y
            };
        } else {
            local_data.spread_offset = local_data.spread.len();
        }

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

        if is_titan && should_shoot {
            use crate::bots::TitanClass as TC;

            if let TC::Northstar = local_data.titan {
                if !should_charge {
                    // horrible!!!
                    cmd.buttons &= !(Action::Attack as u32);
                    cmd.buttons |=
                        Action::Attack as u32 * (helper.globals.frameCount % 8 >= 4) as u32;
                    cmd.buttons |= Action::Zoom as u32;
                } else {
                    cmd.buttons |= Action::Zoom as u32;
                    cmd.buttons &= !(Action::Attack as u32);
                }
            }

            cmd.buttons |= match (local_data.counter, local_data.titan) {
                (_, TC::Scorch) if distance(origin, target) <= 200. => Action::OffHand0 as u32,
                (0..60, TC::Scorch) => 0,
                (60..120, TC::Ronin | TC::Ion) => 0,
                (120..180, TC::Legion) => 0,
                (0..60, _) => Action::OffHand0 as u32,
                (60..120, _) => Action::OffHand1 as u32,
                (120..180, _) => Action::OffHand2 as u32,
                (180..240, _) => Action::OffHand3 as u32,
                (240..300, _) if should_shoot => {
                    local_data.counter = 0;
                    Action::OffHand4 as u32 // core
                }
                (240.., _) => {
                    local_data.counter = 0;
                    0
                }
            };
            local_data.counter += 1;
        }
    } else {
        cmd.buttons |= Action::Reload as u32;

        cmd.world_view_angles.x = 0.;
    }

    let actions = [
        Action::OffHand0,
        Action::OffHand1,
        Action::OffHand2,
        Action::OffHand3,
        Action::OffHand4,
        Action::OffhandQuick,
    ];
    if actions
        .into_iter()
        .map(|action| action as u32)
        .any(|action| cmd.buttons & action != 0)
        && !is_titan
    {
        cmd.buttons &= !(Action::Attack as u32);
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

        local_data.next_check = helper.globals.curTime;
    }

    cmd.camera_angles = cmd.world_view_angles;

    cmd
}

fn find_closest_battery(pos: Vector3, helper: &CUserCmdHelper) -> Option<Vector3> {
    let token = try_refresh_headhunter(helper);

    unsafe { &*HEADHUNTER_DATA.get(token).get() }
        .batteries
        .iter()
        .map(|this| (*this, distance(pos, *this)))
        .reduce(|closer, other| if closer.1 < other.1 { closer } else { other })
        .map(|(pos, _)| pos)
}

fn find_closest_hardpoint(pos: Vector3, helper: &CUserCmdHelper) -> Vector3 {
    let token = try_refresh_headhunter(helper);

    unsafe { &*HEADHUNTER_DATA.get(token).get() }
        .hardpoints
        .iter()
        .map(|this| (*this, distance3(pos, *this)))
        .reduce(|closer, other| if closer.1 < other.1 { closer } else { other })
        .map(|(pos, _)| pos)
        .unwrap_or_else(|| {
            log::warn!("no hardpoints found");
            Vector3::ZERO
        })
}

fn find_flag_for(
    team: i32,
    match_team: bool,
    player_index: u32,
    helper: &CUserCmdHelper,
) -> (Vector3, bool) {
    let token = try_refresh_ctf(helper);

    unsafe { &*CTF_DATA.get(token).get() }
        .flags
        .iter()
        .find(|(_, flag_team, _)| (*flag_team == team) == match_team)
        .copied()
        .map(|(pos, _, parent)| (pos, parent == player_index))
        .unwrap_or(Default::default())
}

fn find_base_for(team: i32, match_team: bool, helper: &CUserCmdHelper) -> Vector3 {
    let token = try_refresh_ctf(helper);

    unsafe { &*CTF_DATA.get(token).get() }
        .bases
        .iter()
        .find(|(_, flag_team)| (*flag_team == team) == match_team)
        .copied()
        .map(|(pos, _)| pos)
        .unwrap_or(Default::default())
}

fn try_refresh_headhunter(helper: &CUserCmdHelper) -> EngineToken {
    let token = unsafe { EngineToken::new_unchecked() };
    let data = unsafe { &mut *HEADHUNTER_DATA.get(token).get() };
    let mut v = Vector3::ZERO;

    if data.last_checked == helper.globals.frameCount {
        return token;
    }
    data.last_checked = helper.globals.frameCount;

    data.batteries.clear();
    data.hardpoints.clear(); // hmm

    data.batteries.extend(
        get_ents_by_class_name(c"item_titan_battery", helper.sv_funcs)
            .filter(|ent| unsafe { (helper.sv_funcs.get_parent)((*ent).cast_const()).is_null() })
            .map(|ent| unsafe {
                // the vtable is the almost the same so it's safe
                *ent.cast::<CPlayer>()
                    .as_ref()
                    .unwrap_unchecked()
                    .get_origin(&mut v)
            }),
    );
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

fn try_refresh_ctf(helper: &CUserCmdHelper) -> EngineToken {
    let token = unsafe { EngineToken::new_unchecked() };
    let data = unsafe { &mut *CTF_DATA.get(token).get() };
    let mut v = Vector3::ZERO;

    if data.last_checked == helper.globals.frameCount {
        return token;
    }
    data.last_checked = helper.globals.frameCount;

    data.bases.clear();
    data.flags.clear(); // hmm

    data.bases.extend(
        get_ents_by_class_name(c"info_spawnpoint_flag", helper.sv_funcs).map(|ent| unsafe {
            (
                *ent.as_ref().unwrap_unchecked().get_origin(&mut v),
                ent.as_ref().unwrap_unchecked().m_iTeamNum,
            )
        }),
    );
    data.flags.extend(
        get_ents_by_class_name(c"item_flag", helper.sv_funcs).map(|ent| unsafe {
            (
                *ent.as_ref().unwrap_unchecked().get_origin(&mut v),
                ent.as_ref().unwrap_unchecked().m_iTeamNum,
                (helper.sv_funcs.get_parent)(ent.cast_const())
                    .as_ref()
                    .and_then::<&CPlayer, _>(|parent| parent.dynamic_cast())
                    .map(|parent| parent.pl.index as u32)
                    .unwrap_or(u32::MAX),
            )
        }),
    );

    token
}

// fn generate_spread(dir: Vector3) -> Vec<Vector3> {
//     dbg!("wow");
//     let dir = normalize(dir);
//     (0..20i32)
//         .map(|i| i.saturating_sub(1) as f32)
//         .map(|i| dir * Vector3::new(i, i, 0.))
//         .map(|angle| Vector3::new(angle.x.clamp(-10., 10.), angle.y.clamp(-10., 10.), 0.))
//         .collect()
// }

#[allow(dead_code)]
fn generate_spread(dir: Vector3, buffer: &mut [Vector3]) {
    dbg!("wow");
    let dir = normalize(dir);
    let spread = (0..50)
        .map(|i| (i as f32 / 25.))
        .map(|i| dir * Vector3::new(i, i, 0.))
        .map(|angle| Vector3::new(angle.x.clamp(-4., 4.), angle.y.clamp(-4., 4.), 0.));

    buffer
        .iter_mut()
        .zip(spread)
        .for_each(|(buffer, offset)| *buffer = offset);
}
