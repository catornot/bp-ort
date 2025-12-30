use bevy_math::prelude::*;
use bonsai_bt::Status;
use rrplug::{
    bindings::class_types::{
        cbaseentity::CBaseEntity,
        cplayer::{CPlayer, EHandle},
    },
    prelude::*,
};
use rustc_hash::FxHasher;
use shared::{
    bindings::{Action as MoveAction, Contents, TraceCollisionGroup},
    cmds_helper::CUserCmdHelper,
    utils::{get_entity_handle, get_player_index, is_alive, lookup_ent, nudge_type, trace_ray},
};
use std::{
    collections::BTreeMap,
    f32::consts::{PI, TAU},
    hash::{Hash, Hasher},
};

use crate::behavior::BotBrain;
use crate::{async_pathfinding::GoalFloat, behavior::BotAction};

#[derive(Debug, Clone)]
pub struct Targeting {
    pub current_target: Target,
    pub last_target: Target,
    pub reacts_at: f32,
    pub spread: Vec<Vector3>,
    pub spread_rigth: bool,
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
    Area(Vector3, f64),
    Roam,
    None,
}

impl PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Target::Entity(this, _), Target::Entity(other, _)) => this == other,
            (Target::Position(this), Target::Position(other)) => this == other,
            (Target::Area(this, _), Target::Area(other, _)) => this == other,
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

    pub fn to_goal(self, helper: &CUserCmdHelper) -> Option<GoalFloat> {
        match self {
            Target::Entity(_, _) => Some(GoalFloat::Point(self.to_position(helper)?)),
            Target::Position(pos) => Some(GoalFloat::ClosestToPoint(pos)),
            Target::Area(pos, radius) => Some(GoalFloat::Area(pos, radius)),
            Target::Roam => Some(GoalFloat::Distance(15)),
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
        TargetingAction::FindTarget => {
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
                        && let trace = trace_ray(
                            brain.origin,
                            Vector3::from(left.0.to_array()),
                            Some(bot),
                            TraceCollisionGroup::BlockWeaponsAndPhysics,
                            Contents::SOLID
                                | Contents::MOVEABLE
                                | Contents::WINDOW
                                | Contents::MONSTER
                                | Contents::GRATE
                                | Contents::PLAYER_CLIP,
                            helper.sv_funcs,
                            helper.engine_funcs,
                        )
                        && (trace.hit_ent == left.1 || trace.fraction == 1.0)
                    {
                        Some(left)
                    } else if left.is_none()
                        && let trace = trace_ray(
                            brain.origin,
                            Vector3::from(rigth.0.to_array()),
                            Some(bot),
                            TraceCollisionGroup::BlockWeaponsAndPhysics,
                            Contents::SOLID
                                | Contents::MOVEABLE
                                | Contents::WINDOW
                                | Contents::MONSTER
                                | Contents::GRATE
                                | Contents::PLAYER_CLIP,
                            helper.sv_funcs,
                            helper.engine_funcs,
                        )
                        && (trace.hit_ent == rigth.1 || trace.fraction == 1.0)
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
                brain.t.current_target = Target::Roam;
                return (Status::Success, 0.);
            };
            brain.t.current_target = Target::Entity(current_target.0, current_target.1);

            (Status::Success, 0.)
        }

        TargetingAction::TargetSwitching => {
            if brain.t.current_target != brain.t.last_target {
                brain.needs_new_path = true;
                brain.path_receiver = None; // honestly should figure smth better

                brain.t.last_target = brain.t.current_target;
                brain.t.spread.clear();
            }

            (Status::Success, 0.)
        }

        TargetingAction::Shoot => {
            if let Target::Entity(handle, true) = brain.t.current_target
                && let Some(ent) = lookup_ent(handle, helper.sv_funcs)
            {
                if helper.globals.curTime > brain.t.reacts_at {
                    let mut v = Vector3::ZERO;

                    if brain.t.spread.is_empty() {
                        generate_spread(
                            &mut brain.t.spread,
                            brain.t.spread_rigth,
                            helper.globals.tickCount as u64 + get_player_index(bot) as u64,
                        );
                        brain.t.spread_rigth = !brain.t.spread_rigth;
                    }

                    brain.next_cmd.buttons |= MoveAction::Attack as u32 | MoveAction::Zoom as u32;
                    brain.next_cmd.world_view_angles = natural_aim(
                        brain.angles,
                        look_at(brain.origin, unsafe { *ent.get_origin(&mut v) })
                            + brain.t.spread.pop().unwrap_or_default(),
                    );
                    // brain.next_cmd.world_view_angles =
                    //     look_at(brain.origin, unsafe { *ent.get_origin(&mut v) })
                    //         + brain.t.spread.pop().unwrap_or_default();

                    let mut v = Vector3::ZERO;
                    if let Some(point) = brain.path.get(1)
                        && trace_ray(
                            unsafe { *ent.get_origin(&mut v) },
                            point.as_vec(),
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
                        .fraction
                            < 1.
                    {
                        brain.m.can_move = false;
                    }

                    brain.m.view_lock = true;
                }
            } else if let Target::Entity(handle, false) = brain.t.current_target
                && let Target::Entity(last_handle, true) = brain.t.last_target
                && handle == last_handle
                && let Some(ent) = lookup_ent(handle, helper.sv_funcs)
                && !is_alive(ent)
            {
                brain.next_cmd.buttons |= MoveAction::Reload as u32;
            } else if let Target::Entity(handle, false) = brain.t.last_target
                && let Some(ent) = lookup_ent(handle, helper.sv_funcs)
                && !is_alive(ent)
            {
                brain.next_cmd.buttons |= MoveAction::Reload as u32;
            }
            // maybe add a check if ammo isn't full? then reload

            if let Target::Entity(_, false) = brain.t.current_target {
                const REACTON_TIME: f32 = 0.4;
                brain.t.reacts_at = helper.globals.curTime + REACTON_TIME;
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

fn generate_spread(spread_buf: &mut Vec<Vector3>, spread_rigth: bool, seed: u64) {
    const VALUES: usize = 30;
    const VALUES_BOTTOM: i32 = VALUES as i32 / -3;
    const VALUES_TOP: i32 = VALUES as i32 * 2 / 3;
    const VALUES_STEP: f32 = 0.2;
    const PERMUTATIONS: u64 = 15;
    const Y_FLUTUATION_PERMUTATIONS: u64 = 10;
    const Y_FLUTUATION_PERMUTATIONS_MIN: u64 = 1;
    const Y_FLUTUATION_FRAC: f32 = 0.01;

    spread_buf.clear();

    let rigth_way = (VALUES_BOTTOM..VALUES_TOP).filter(|_| spread_rigth);
    let left_way = (-VALUES_TOP..-VALUES_BOTTOM)
        .filter(|_| !spread_rigth)
        .rev();

    let mut hasher = FxHasher::default();
    seed.hash(&mut hasher);
    let angle = (hasher.finish() % PERMUTATIONS) as f32 / PERMUTATIONS as f32;
    spread_buf.extend(
        rigth_way
            .chain(left_way)
            .map(|i| {
                let mut hasher = FxHasher::default();
                seed.hash(&mut hasher);
                Y_FLUTUATION_PERMUTATIONS.hash(&mut hasher);
                (
                    i as f32,
                    ((hasher.finish() % Y_FLUTUATION_PERMUTATIONS)
                        .min(Y_FLUTUATION_PERMUTATIONS_MIN) as f32
                        / Y_FLUTUATION_PERMUTATIONS as f32
                        - Y_FLUTUATION_PERMUTATIONS as f32 / 2.)
                        * Y_FLUTUATION_FRAC,
                )
            })
            .map(|(x, fluctuation)| {
                Vector3::new(angle * x * VALUES_STEP - fluctuation, x * VALUES_STEP, 0.)
            }),
    );
}

const AIM_DELTA: f32 = PI / 20.;
pub fn natural_aim(current: Vector3, target: Vector3) -> Vector3 {
    Vector3::new(
        angle_move_toward(current.x, target.x, AIM_DELTA),
        angle_move_toward(current.y, target.y, AIM_DELTA),
        angle_move_toward(current.z, target.z, AIM_DELTA),
    )
}

fn angle_move_toward(from: f32, to: f32, delta: f32) -> f32 {
    let (from, to) = (from.to_radians(), to.to_radians());
    let diff = angle_diff(from, to);
    // When `delta < 0` move no further than to PI radians away from `to` (as PI is the max possible angle distance).
    (from + delta.clamp(diff.abs() - PI, diff.abs()) * if diff >= 0.0 { 1. } else { -1. })
        .to_degrees()
}
fn angle_diff(from: f32, to: f32) -> f32 {
    let diff = (to - from).rem_euclid(TAU);
    (2.0 * diff).rem_euclid(TAU) - diff
}
