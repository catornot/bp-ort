use std::{collections::BTreeMap, ptr};

use bonsai_bt::Status;
use rrplug::{
    bindings::class_types::cplayer::{CPlayer, EHandle},
    prelude::*,
};
use shared::{
    cmds_helper::CUserCmdHelper,
    utils::{get_entity_handle, get_ents_by_class_name, lookup_ent},
};

use crate::{
    behavior::{BotAction, BotBrain},
    targeting::{Target, TargetingMode},
};

#[derive(Debug, Clone, Default)]
pub struct SharedGamemodeCTF {
    last_updated: f32,
    flags: BTreeMap<i32, Flag>,
    bases: BTreeMap<i32, Base>,
}

#[derive(Debug, Clone)]
pub enum GamemodeCTFAction {
    UpdateGamemodeState,
    DetermineTarget,
}

#[derive(Debug, Clone, Default)]
pub struct Flag {
    origin: Vector3,
    team: i32,
    parent: EHandle,
}

#[derive(Debug, Clone, Default)]
pub struct Base {
    origin: Vector3,
}

impl From<GamemodeCTFAction> for BotAction {
    fn from(val: GamemodeCTFAction) -> Self {
        BotAction::GamemodeCTF(val)
    }
}

pub fn run_ctf(
    cp: &GamemodeCTFAction,
    brain: &mut BotBrain,
    bot: &CPlayer,
    helper: &CUserCmdHelper,
) -> (Status, f64) {
    match cp {
        GamemodeCTFAction::UpdateGamemodeState => {
            let mut shared = brain.shared.lock();
            let ctf = &mut shared.ctf;
            if ctf.last_updated != helper.globals.curTime {
                ctf.last_updated = helper.globals.curTime;

                const OFFSET: Vector3 = Vector3::new(0., 0., 100.);
                // log::info!("collecting data");
                let mut v = Vector3::ZERO;
                ctf.flags.clear();
                ctf.flags.extend(
                    get_ents_by_class_name(c"item_flag", helper.sv_funcs)
                        .flat_map(|ent| unsafe { ent.as_ref() })
                        .map(|ent| {
                            (
                                ent.m_iTeamNum,
                                Flag {
                                    origin: unsafe { *ent.get_origin(&mut v) } + OFFSET,
                                    team: ent.m_iTeamNum,
                                    parent: ent.m_hMoveParent,
                                },
                            )
                        }),
                );

                ctf.bases.clear();
                ctf.bases.extend(
                    get_ents_by_class_name(c"info_spawnpoint_flag", helper.sv_funcs)
                        .flat_map(|ent| unsafe { ent.as_ref() })
                        .map(|ent| {
                            (
                                ent.m_iTeamNum,
                                Base {
                                    origin: unsafe { *ent.get_origin(&mut v) } + OFFSET,
                                },
                            )
                        }),
                )
            }
            (Status::Success, 0.)
        }
        // TODO: convert this into a behavior tree
        GamemodeCTFAction::DetermineTarget => {
            let mut shared = brain.shared.lock();
            let ctf = &mut shared.ctf;

            let mut v = Vector3::ZERO;
            if let Some(target_flag) = ctf
                .flags
                .get(&get_other_team(bot.m_iTeamNum))
                .filter(|flag| flag.parent == get_entity_handle(bot))
                .or_else(|| {
                    ctf.flags.get(&bot.m_iTeamNum).filter(|our_flag| {
                        !get_team_player_origin(bot, helper, &mut v)
                            .any(|origin| distance3(our_flag.origin, origin) < 500.)
                            || ctf
                                .bases
                                .get(&bot.m_iTeamNum)
                                .filter(|base| distance3(base.origin, our_flag.origin) < 50.)
                                .is_none()
                    })
                })
                .or_else(|| ctf.flags.get(&get_other_team(bot.m_iTeamNum)))
            {
                brain.t.mode = TargetingMode::PassbyAgressive;

                if brain.path_receiver.is_none()
                    && brain.path_next_request + 0.1 < helper.globals.curTime
                    && target_flag.parent != get_entity_handle(bot)
                {
                    match (
                        brain
                            .path
                            .back()
                            .filter(|point| distance3(point.as_vec(), target_flag.origin) < 100.)
                            .is_some(),
                        brain
                            .path
                            .back()
                            .filter(|point| distance3(point.as_vec(), target_flag.origin) < 250.)
                            .is_some(),
                    ) {
                        (true, true) => {}
                        (true, false) => log::info!("this is suppose to be unreachable how lol"),
                        // HACK: clear path if it's too far
                        (false, true) | (false, false) => {
                            brain.path_receiver = None;
                            brain.path_next_request = helper.globals.curTime + 0.2;
                            brain.path.drain(brain.path.len().min(10)..);
                            brain.needs_new_path = true;
                        }
                    }
                }

                if target_flag.team != bot.m_iTeamNum
                    || !matches!(brain.t.current_target, Target::Entity(_, _))
                {
                    if target_flag.parent != get_entity_handle(bot) {
                        brain.t.current_target = if distance3(target_flag.origin, brain.origin)
                            < 100.
                            && target_flag.team == bot.m_iTeamNum
                        {
                            Target::Roam
                        } else if target_flag.team == bot.m_iTeamNum
                            && ctf
                                .bases
                                .get(&bot.m_iTeamNum)
                                .filter(|base| distance3(base.origin, target_flag.origin) < 50.)
                                .is_none()
                            && let Some(_) = lookup_ent(target_flag.parent, helper.sv_funcs)
                        {
                            Target::Entity(target_flag.parent, false)
                        } else {
                            Target::Position(target_flag.origin)
                        };
                    } else {
                        // go back to the base
                        log::info!("going back home");
                        brain.t.current_target = Target::Position(
                            ctf.bases
                                .get(&get_other_team(target_flag.team))
                                .map(|base| base.origin)
                                .unwrap_or_default()
                                + Vector3::new(0., 0., 100.),
                        );
                    }
                }
            } else {
                brain.t.mode = TargetingMode::Agressive;
            }

            (Status::Success, 0.)
        }
    }
}

fn get_team_player_origin<'a>(
    bot: &'a CPlayer,
    helper: &'a CUserCmdHelper<'_>,
    v: &'a mut Vector3,
) -> impl Iterator<Item = Vector3> + 'a {
    (0..helper.globals.maxPlayers)
        .flat_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i).as_ref() })
        .filter(|player| !ptr::eq(*player, bot) && player.m_iTeamNum == bot.m_iTeamNum)
        .map(|player| unsafe { *player.get_origin(v) })
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
