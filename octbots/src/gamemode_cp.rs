use std::{collections::HashMap, ptr};

use bonsai_bt::Status;
use rrplug::{
    bindings::class_types::{
        cbaseentity::CBaseEntity,
        cplayer::{CPlayer, EHandle},
    },
    prelude::*,
};
use shared::{
    cmds_helper::CUserCmdHelper,
    utils::{
        get_entity_handle, get_ents_by_class_name, get_global_net_float, get_global_net_int,
        get_value_for_key_string,
    },
};

use crate::{
    behavior::{BotAction, BotBrain},
    targeting::{Target, TargetingMode},
};

#[derive(Debug, Clone, Default)]
pub struct SharedGamemodeCP {
    last_updated: f32,
    capture_point_claims: Vec<HashMap<char, Option<EHandle>>>,
    capture_points: Vec<CapturePoint>,
}

#[derive(Debug, Clone, Default)]
pub struct GamemodeCP {}

#[derive(Debug, Clone)]
pub enum GamemodeCPAction {
    UpdateGamemodeState,
    DetermineTarget,
    RemoveTarget,
}

#[derive(Debug, Clone, Default)]
pub struct CapturePoint {
    origin: Vector3,
    cap_frac: f32,
    cap_team: i32,
    team: i32,
    group: char,
}

impl From<GamemodeCPAction> for BotAction {
    fn from(val: GamemodeCPAction) -> Self {
        BotAction::GamemodeCP(val)
    }
}

pub fn run_cp(
    cp: &GamemodeCPAction,
    brain: &mut BotBrain,
    bot: &CPlayer,
    helper: &CUserCmdHelper,
) -> (Status, f64) {
    match cp {
        GamemodeCPAction::UpdateGamemodeState => {
            let mut shared = brain.shared.lock();
            let cp = &mut shared.cp;
            if cp.last_updated != helper.globals.curTime {
                cp.last_updated = helper.globals.curTime;

                const OFFSET: Vector3 = Vector3::new(0., 0., 100.);
                // log::info!("collecting data");
                let mut v = Vector3::ZERO;
                cp.capture_points.clear();
                cp.capture_points.extend(
                    get_ents_by_class_name(c"info_hardpoint", helper.sv_funcs)
                        .flat_map(|ent| unsafe { ent.as_ref() })
                        .map(|ent| CapturePoint {
                            origin: unsafe { *ent.get_origin(&mut v) } + OFFSET,
                            // cap_frac: capture_point_get_capture_progress(ent, helper),
                            // cap_team: capture_point_get_get_capping_team(ent, helper),
                            team: ent.m_iTeamNum,
                            group: capture_point_get_group(ent).chars().next().unwrap_or('\0'),
                            cap_frac: 0.,
                            cap_team: 0,
                        }),
                )
            }
            (Status::Success, 0.)
        }
        GamemodeCPAction::DetermineTarget => 'label: {
            let mut shared = brain.shared.lock();
            let cp = &mut shared.cp;

            let mut v = Vector3::ZERO;
            if cp.capture_point_claims.len()
                <= get_other_team(bot.m_iTeamNum).max(bot.m_iTeamNum) as usize
            {
                cp.capture_point_claims.extend(
                    (cp.capture_point_claims.len()..bot.m_iTeamNum as usize + 1)
                        .map(|_| Default::default()),
                );
                break 'label (Status::Success, 0.);
            }

            if let Some(closest) = cp
                .capture_points
                .iter()
                .fold(None, |first: Option<&CapturePoint>, second| {
                    if let Some(first) = first
                        && distance3(first.origin, brain.abs_origin)
                            <= distance3(second.origin, brain.abs_origin)
                        && second.team != bot.m_iTeamNum
                    {
                        Some(first)
                    } else if cp.capture_point_claims[bot.m_iTeamNum as usize]
                        .entry(second.group)
                        .or_default()
                        .as_ref()
                        == Some(&get_entity_handle(bot))
                        || second.team != bot.m_iTeamNum
                    {
                        Some(second)
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    cp.capture_points
                        .iter()
                        .fold(None, |first: Option<&CapturePoint>, second| {
                            if !(0..helper.globals.maxPlayers)
                                .flat_map(|i| unsafe {
                                    (helper.sv_funcs.get_player_by_index)(i).as_ref()
                                })
                                .filter(|player| {
                                    !ptr::eq(*player, bot) && player.m_iTeamNum == bot.m_iTeamNum
                                })
                                .map(|player| unsafe { *player.get_origin(&mut v) })
                                .any(|origin| {
                                    first
                                        .map(|first| distance3(first.origin, origin))
                                        .unwrap_or(f32::MAX)
                                        .min(distance3(second.origin, origin))
                                        < 300.
                                })
                            {
                                if let Some(first) = first
                                    && distance3(first.origin, brain.abs_origin)
                                        <= distance3(second.origin, brain.abs_origin)
                                {
                                    Some(first)
                                } else {
                                    Some(second)
                                }
                            } else {
                                None
                            }
                        })
                })
            {
                brain.t.mode = TargetingMode::PassbyAgressive;
                cp.capture_point_claims[bot.m_iTeamNum as usize]
                    .iter_mut()
                    .for_each(|(_, set)| {
                        _ = set.filter(|handle| *handle != get_entity_handle(bot))
                    });

                if distance3(brain.origin, closest.origin) < 300. {
                    _ = cp.capture_point_claims[bot.m_iTeamNum as usize]
                        .entry(closest.group)
                        .or_default()
                        .insert(get_entity_handle(bot));
                }
                brain.t.current_target = Target::Position(closest.origin);
            } else {
                cp.capture_point_claims[bot.m_iTeamNum as usize]
                    .iter_mut()
                    .for_each(|(_, set)| {
                        _ = set.filter(|handle| *handle != get_entity_handle(bot))
                    });
                brain.t.mode = TargetingMode::Agressive;
            }

            (Status::Success, 0.)
        }
        GamemodeCPAction::RemoveTarget => {
            let mut shared = brain.shared.lock();
            shared
                .cp
                .capture_point_claims
                .iter_mut()
                .for_each(|capture_points| {
                    capture_points.iter_mut().for_each(|players| {
                        _ = players.1.filter(|index| *index != get_entity_handle(bot))
                    })
                });
            (Status::Success, 0.)
        }
    }
}

fn capture_point_get_group(hardpoint: &CBaseEntity) -> String {
    // log::info!("hardpointGroup");
    get_value_for_key_string(hardpoint, c"hardpointGroup").unwrap_or_else(|| "B".to_string())
}

fn capture_point_get_get_capping_team(hardpoint: &CBaseEntity, helper: &CUserCmdHelper) -> i32 {
    log::info!("capping");
    get_global_net_int(
        "objective".to_string() + &capture_point_get_group(hardpoint) + "CappingTeam",
        helper.sv_funcs,
    )
}

fn capture_point_get_capture_progress(hardpoint: &CBaseEntity, helper: &CUserCmdHelper) -> f32 {
    log::info!("progress");
    get_global_net_float(
        "objective".to_string() + &capture_point_get_group(hardpoint) + "Progress",
        helper.sv_funcs,
    )
}

fn get_other_team(team: i32) -> i32 {
    if team == 3 {
        2
    } else {
        3
    }
}

pub fn distance3(pos: Vector3, target: Vector3) -> f32 {
    ((pos.x - target.x).powi(2) + (pos.y - target.y).powi(2) + (pos.z - target.z).powi(2)).sqrt()
}
