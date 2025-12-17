use bevy_math::prelude::*;
use bonsai_bt::Status;
use rrplug::{
    bindings::class_types::{
        cbaseentity::CBaseEntity,
        cplayer::{CPlayer, EHandle},
    },
    prelude::*,
};
use shared::{
    bindings::{Action as MoveAction, Contents, TraceCollisionGroup},
    cmds_helper::CUserCmdHelper,
    utils::{get_entity_handle, get_player_index, is_alive, lookup_ent, nudge_type, trace_ray},
};
use std::collections::BTreeMap;

use crate::behavior::BotAction;
use crate::behavior::BotBrain;

#[derive(Debug, Clone)]
pub struct Targeting {
    pub current_target: Target,
    pub last_target: Target,
    pub hates: BTreeMap<usize, u32>,
}

#[derive(Debug, Clone)]
pub enum TargetingAction {
    FindTarget,
    TargetSwitching,
    Shoot,
}

#[derive(Debug, Clone, Copy)]
pub enum Target {
    Entity(EHandle, bool),
    Position(Vector3),
    Area(Vector3, f32),
    Roam,
    None,
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Target::Entity(this, _), Target::Entity(other, _)) => this == other,
            (Target::Position(this), Target::Position(other)) => this == other,
            (Target::Area(this, _), Target::Area(other, _)) if this == other => todo!(),
            (Target::Roam, Target::Roam) => true,
            (Target::None, Target::None) => true,
            _ => false,
        }
    }
}

impl Target {
    pub fn to_position(self, helper: &CUserCmdHelper) -> Option<Vector3> {
        match self {
            Target::Entity(handle, _) => lookup_ent(handle, helper.sv_funcs).map(|ent| unsafe {
                let mut v = Vector3::ZERO;
                *ent.get_origin(&mut v)
            }),
            Target::Position(pos) => Some(pos),
            Target::Area(pos, _) => Some(pos),
            Target::Roam => None,
            Target::None => None,
        }
    }
}

impl From<TargetingAction> for BotAction {
    fn from(val: TargetingAction) -> Self {
        BotAction::Targeting(val)
    }
}

pub fn run_targeting(
    targeting: &TargetingAction,
    brain: &mut BotBrain,
    bot: &CPlayer,
    helper: &CUserCmdHelper,
) -> (Status, f64) {
    match targeting {
        TargetingAction::FindTarget => 'target: {
            let base = Vec3::new(brain.origin.x, brain.origin.y, brain.origin.z);

            fn make_player_iterator<'a>(
                bot: &'a CBaseEntity,
                helper: &CUserCmdHelper<'a>,
            ) -> impl Iterator<Item = (Vec3, &'a CBaseEntity, usize, i32)> + 'a {
                (0..helper.globals.maxPlayers)
                    .flat_map(|i| unsafe { (helper.sv_funcs.get_player_by_index)(i + 1).as_ref() })
                    .filter(|other| {
                        get_entity_handle(other) != get_entity_handle(bot)
                            && other.m_iTeamNum != bot.m_iTeamNum
                            && is_alive(other)
                    })
                    .map(|other| {
                        let mut v = Vector3::ZERO;
                        (
                            unsafe { *other.get_origin(&mut v) },
                            other,
                            get_player_index(other),
                            get_entity_handle(other),
                        )
                    })
                    .map(|(Vector3 { x, y, z }, ent, index, handle)| {
                        (
                            Vec3::new(x, y, z),
                            nudge_type::<&CBaseEntity>(ent),
                            index,
                            handle,
                        )
                    })
            }

            let Some(current_target) = make_player_iterator(bot, helper)
                .fold(None::<(Vec3, &CBaseEntity, usize, i32)>, |left, rigth| {
                    if let Some(left) = left
                        && (left.0.distance(base) as u32).saturating_sub(
                            brain.t.hates.get(&left.2).copied().unwrap_or_default() * 50,
                        ) < (rigth.0.distance(base) as u32).saturating_sub(
                            brain.t.hates.get(&rigth.2).copied().unwrap_or_default() * 50,
                        )
                        && std::ptr::eq(
                            trace_ray(
                                brain.origin,
                                Vector3::from(left.0.to_array()),
                                Some(bot),
                                TraceCollisionGroup::None,
                                Contents::SOLID
                                    | Contents::MOVEABLE
                                    | Contents::WINDOW
                                    | Contents::MONSTER
                                    | Contents::GRATE
                                    | Contents::PLAYER_CLIP,
                                helper.sv_funcs,
                                helper.engine_funcs,
                            )
                            .hit_ent,
                            left.1,
                        )
                    {
                        Some(left)
                    } else if left.is_none()
                        && std::ptr::eq(
                            trace_ray(
                                brain.origin,
                                Vector3::from(rigth.0.to_array()),
                                Some(bot),
                                TraceCollisionGroup::None,
                                Contents::SOLID
                                    | Contents::MOVEABLE
                                    | Contents::WINDOW
                                    | Contents::MONSTER
                                    | Contents::GRATE
                                    | Contents::PLAYER_CLIP,
                                helper.sv_funcs,
                                helper.engine_funcs,
                            )
                            .hit_ent,
                            rigth.1,
                        )
                    {
                        Some(rigth)
                    } else {
                        None
                    }
                })
                .map(|(_, _, _, handle)| (handle, true))
                .or_else(|| {
                    make_player_iterator(bot, helper)
                        .reduce(|left, rigth| {
                            if (left.0.distance(base) as u32).saturating_sub(
                                brain.t.hates.get(&left.2).copied().unwrap_or_default() * 50,
                            ) < (rigth.0.distance(base) as u32).saturating_sub(
                                brain.t.hates.get(&rigth.2).copied().unwrap_or_default() * 50,
                            ) {
                                left
                            } else {
                                rigth
                            }
                        })
                        .map(|(_, _, _, handle)| (handle, false))
                })
            else {
                break 'target (Status::Failure, 0.);
            };
            brain.t.current_target = Target::Entity(current_target.0, current_target.1);

            (Status::Success, 0.)
        }

        TargetingAction::TargetSwitching => {
            if brain.t.current_target != brain.t.last_target {
                brain.t.last_target = brain.t.current_target;
                brain.needs_new_path = true;
                brain.path_receiver = None; // honestly should figure smth better
            }

            (Status::Success, 0.)
        }

        TargetingAction::Shoot => {
            if let Target::Entity(handle, true) = brain.t.current_target
                && let Some(ent) = lookup_ent(handle, helper.sv_funcs)
            {
                let mut v = Vector3::ZERO;

                brain.next_cmd.buttons |= MoveAction::Attack as u32 | MoveAction::Zoom as u32;
                brain.next_cmd.world_view_angles =
                    look_at(brain.origin, unsafe { *ent.get_origin(&mut v) });
                brain.m.can_move = true;
                brain.m.view_lock = true;
            } else {
                brain.next_cmd.buttons |= MoveAction::Reload as u32;
            }

            (Status::Success, 0.)
        }
    }
}

pub fn look_at(origin: Vector3, target: Vector3) -> Vector3 {
    let diff = target - origin;
    let angley = diff.y.atan2(diff.x).to_degrees();
    let anglex = diff
        .z
        .atan2((diff.x.powi(2) + diff.y.powi(2)).sqrt())
        .to_degrees();

    Vector3::new(-anglex, angley, 0.)
}
