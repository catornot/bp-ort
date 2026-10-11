use bevy_math::{bounding::RayCast3d, prelude::*};
use bonsai_bt::Status;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use rrplug::{
    bindings::server::{
        EHandle,
        cbaseentity::CBaseEntity,
        cplayer::CPlayer,
        cweaponx::{CWeaponX, WeaponState},
    },
    high::UnsafeHandle,
    prelude::*,
};
use rustc_hash::FxHasher;
use shared::{
    bindings::{
        Action as MoveAction, Contents, ENGINE_FUNCTIONS, SERVER_FUNCTIONS, TraceCollisionGroup,
    },
    cmds_helper::CUserCmdHelper,
    utils::{
        get_entity_handle, get_eye_position, get_npc_buffer, get_player_index, is_alive,
        lookup_ent, nudge_type, trace_ray,
    },
};
use std::{
    array,
    cmp::Ordering,
    collections::{BTreeMap, BinaryHeap},
    f32::consts::{PI, TAU},
    fmt::Debug,
    hash::{Hash, Hasher},
    ops::Sub as _,
    sync::atomic::AtomicU8,
    sync::atomic::Ordering as AtomicOrdering,
};

use crate::{
    async_pathfinding::GoalFloat, behavior::BotAction, difficulty::TargetingConfig,
    gamemode_cp::distance3,
};
use crate::{
    behavior::{BotBrain, SharedBotBrain},
    loader::NavmeshStatus,
};

const REACTON_TIME: f32 = 0.4;
const SWITCH_TIME: f32 = 0.2;

#[derive(Debug, Clone, Default)]
pub struct Targeting {
    pub mode: TargetingMode,
    pub move_target: Target,
    shoot_target: Target,
    last_target: Target,
    reacts_at: f32,
    hates: BTreeMap<usize, u32>,
    possible_targets: BinaryHeap<PossibleTarget>,
    extra_target: Option<PossibleTarget>,
    real_targets: ConstGenericRingBuffer<Target, 4>,
    aim: AimController,
}

#[derive(Debug, Clone, Default)]
struct AimController {
    last_weapon_state: WeaponState,
    spread: ConstGenericRingBuffer<Vector3, 50>,
    spread_right: bool,
    offset_angle: Vector3,
}

// so the bot gets the target, follows it's camera then maybe overshoots, then checks if there is anything there, if not updates hyp_target with it's current position, repeat

#[derive(Debug, Clone, Default)]
struct PossibleTarget {
    pub handle: EHandle,
    /// how close are the targets eye's on us on x/y
    pub looking: f32,
    /// how close the eyes on x/y
    pub eye_contact: f32,
    /// TODO: need support in rrplug to upcast to CBaseCombatCharacter
    pub shooting: bool,
    pub distance: f32,
    pub is_player: bool,
}

#[derive(Debug, Clone)]
pub enum TargetingAction {
    FindTarget,
    TargetSwitching,
    Shoot,
    Melee,
    OnDeathStartHate,
    UpdatePostTargetting,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Target {
    Entity(EHandle, bool),
    Position(Vector3),
    Area(Vector3, f64),
    Roam,
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum TargetingMode {
    Passive,
    PassbyAgressive,
    #[default]
    Agressive,
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

impl Eq for PossibleTarget {}

impl PartialEq for PossibleTarget {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl Ord for PossibleTarget {
    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            return Ordering::Equal;
        }

        if self.is_player && !other.is_player {
            return Ordering::Greater;
        }

        match other.looking.abs().total_cmp(&other.looking) {
            Ordering::Less
                if self.distance.sub(500.).max(0.) >= other.distance.sub(500.).max(0.) =>
            {
                Ordering::Less
            }
            Ordering::Greater
                if self.distance.sub(500.).max(0.) <= other.distance.sub(500.).max(0.) =>
            {
                Ordering::Greater
            }
            Ordering::Equal
                if self.distance.sub(500.).max(0.) <= other.distance.sub(500.).max(0.) =>
            {
                Ordering::Equal
            }
            Ordering::Equal => self.distance.total_cmp(&other.distance),
            ord => ord.reverse(),
        }
    }
}

impl PartialOrd for PossibleTarget {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Targeting {
    pub fn add_target(&mut self, target: Target) {
        // current there are only four targets in memory so this is fine, in the future maybe it would wise to use a hashmap or something
        // if let Some(other) = self.real_targets.iter_mut().find(|other| target == **other) {
        //     mem::swap(&mut target, other);
        // } else {
        self.real_targets.enqueue(target);
        // }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CGameTraceSubSet {
    hit_ent: Option<EHandle>,
    fraction: f32,
}

pub fn run_targeting(
    targeting: &TargetingAction,
    brain: &mut BotBrain,
    bot: &CPlayer,
    helper: &CUserCmdHelper,
) -> (Status, f64) {
    match targeting {
        TargetingAction::FindTarget => {
            let raycast_cache = &mut brain.shared.lock().cached_player_raycasts;
            let targeting_data = &mut brain.t;
            let bot_origin = brain.origin;
            let base = Vec3::new(brain.origin.x, brain.origin.y, brain.origin.z);

            let Some(current_target) = matches!(
                targeting_data.mode,
                TargetingMode::Agressive | TargetingMode::PassbyAgressive
            )
            .then(|| {
                loop {
                    let (ent, target_data) =
                        targeting_data.possible_targets.pop().and_then(|target| {
                            Some((lookup_ent(target.handle, helper.sv_funcs)?, target))
                        })?;

                    let mut v = Vector3::ZERO;
                    let trace = raycast_cache
                        .entry((get_entity_handle(bot), get_entity_handle(ent)))
                        .or_insert_with(|| {
                            let trace = trace_ray(
                                get_eye_position(bot),
                                unsafe { *ent.get_eye_position(&mut v) }, // TODO: use eye position too here maybe (requires rrplug support)
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
                            );
                            CGameTraceSubSet {
                                hit_ent: unsafe { trace.hit_ent.as_ref() }.map(get_entity_handle),
                                fraction: trace.fraction,
                            }
                        });

                    if trace.hit_ent == Some(get_entity_handle(ent)) || trace.fraction == 1.0 {
                        if !target_data.is_player
                            && let Some(player) =
                                get_player_in_view(targeting_data, bot, bot_origin, helper, base)
                        {
                            return Some(player);
                        }

                        break Some(target_data.handle);
                    }
                }
            })
            .flatten()
            .map(|handle| (handle, true))
            .or_else(|| {
                brain
                    .t
                    .extra_target
                    .take()
                    .filter(|_| matches!(brain.t.mode, TargetingMode::Agressive))
                    .map(|target| (target.handle, false))
            })
            .or_else(|| {
                make_player_iterator(bot, helper)
                    .filter(|_| matches!(brain.t.mode, TargetingMode::Agressive))
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
            }) else {
                return (Status::Success, 0.);
            };
            brain
                .t
                .add_target(Target::Entity(current_target.0, current_target.1));

            (Status::Success, 0.)
        }

        TargetingAction::TargetSwitching => {
            brain.t.move_target = Target::Roam;
            brain.t.shoot_target = Target::None;

            let mut could_see = false;
            for target in brain.t.real_targets.iter().rev() {
                let can_move_change = matches!(
                    brain.t.move_target,
                    Target::Entity(_, _) | Target::Roam | Target::None
                );
                match target {
                    Target::Entity(_, can_see) if !could_see => {
                        could_see |= can_see;
                        brain.t.shoot_target = *target;
                        if can_move_change {
                            brain.t.move_target = *target
                        }
                    }

                    Target::Position(_) if can_move_change => brain.t.move_target = *target,
                    Target::Area(_, _) if can_move_change => brain.t.move_target = *target,
                    Target::Position(_)
                    | Target::Area(_, _)
                    | Target::Roam
                    | Target::None
                    | Target::Entity(_, _) => {}
                }
            }

            brain.t.real_targets.clear();

            if brain.t.shoot_target != brain.t.last_target {
                brain.needs_new_path = true;
                brain.path_receiver = None; // honestly should figure smth better

                brain.t.last_target = brain.t.shoot_target;
                brain.t.aim.spread.clear();
                brain.t.reacts_at = helper.globals.curTime + SWITCH_TIME;
            }

            (Status::Success, 0.)
        }

        TargetingAction::Shoot => {
            if let Target::Entity(handle, true) = brain.t.shoot_target
                && let Some(ent) = lookup_ent(handle, helper.sv_funcs)
            {
                let enemy_is_titan = unsafe { ent.is_titan() };
                match (enemy_is_titan, brain.is_titan) {
                    (true, true) => brain.next_cmd.weaponselect = 0, // switch to default,
                    (true, false) => brain.next_cmd.weaponselect = 1,
                    (false, true) => brain.next_cmd.weaponselect = 0, // switch to default,
                    (false, false) => brain.next_cmd.weaponselect = 0, // switch to default,
                }

                #[allow(clippy::nonminimal_bool)]
                // one of the if statements inside throws this warning, I am not sure how to fix it, skill issue fr
                if helper.globals.curTime > brain.t.reacts_at {
                    let mut v = Vector3::ZERO;

                    let mut weapon_state = WeaponState::Idle;
                    if let Some(weapon) = lookup_ent(bot.m_inventory.activeWeapon, helper.sv_funcs)
                        .and_then::<&CWeaponX, _>(|ent| ent.dynamic_cast())
                    {
                        // log::info!("weapon.m_weapState {:?}", weapon.m_weapState);

                        let is_charge_weapon = f32::from_ne_bytes(array::from_fn(|i| {
                            // offset for is_charge_weapon
                            weapon.m_modVars[0x31c..0x31c + 4][i]
                        })) != 0.;
                        let semi_auto_allowed_fire = !weapon.m_playerData.m_semiAutoTriggerDown
                            || (weapon.m_weapState == WeaponState::Idle
                                && weapon.m_weapState == brain.t.aim.last_weapon_state);
                        let is_fully_zoomed_in = weapon.m_playerData.m_targetZoomFOV
                                <= weapon.m_playerData.m_curZoomFOV
                                // from some reason the targetZoomFOV can become like 1000 > sometimes
                                || (weapon.m_playerData.m_targetZoomFOV - weapon.m_playerData.m_curZoomFOV).abs()
                                    > 500.;
                        brain.next_cmd.buttons |= if semi_auto_allowed_fire
                            && (is_fully_zoomed_in
                                || get_weapon_charge_fraction(weapon, helper) < 1.0)
                            && (!is_charge_weapon
                                || (is_charge_weapon
                                    && get_weapon_charge_fraction(weapon, helper) < 1.0))
                        {
                            // MoveAction::Zoom as u32
                            MoveAction::Attack as u32 | MoveAction::Zoom as u32
                        } else {
                            MoveAction::Zoom as u32
                        };

                        weapon_state = weapon.m_weapState;
                    } else {
                        brain.next_cmd.buttons |= MoveAction::Zoom as u32;
                        if in_view(
                            brain.origin,
                            brain.angles,
                            ent.m_vecAbsOrigin,
                            const { std::f32::consts::FRAC_PI_2 },
                        ) {
                            brain.next_cmd.buttons |=
                                MoveAction::Attack as u32 | MoveAction::Zoom as u32;
                        }
                    }

                    if brain.t.aim.spread.is_empty() {
                        let config = &brain.shared.lock().config;
                        generate_spread(
                            &mut brain.t.aim.spread,
                            brain.t.aim.spread_right,
                            &config.targeting,
                            helper.globals.tickCount as u64 + get_player_index(bot) as u64,
                        );
                        brain.t.aim.spread_right = !brain.t.aim.spread_right;
                    }

                    // log::info!("{:?} {:?}", brain.angles, brain.t.aim.offset_angle);
                    // if (brain.angles.x - brain.t.aim.offset_angle.x)
                    //     .abs()
                    //     .rem_euclid(360.)
                    //     < f32::EPSILON * 3.
                    //     && (brain.angles.y - brain.t.aim.offset_angle.y)
                    //         .abs()
                    //         .rem_euclid(360.)
                    //         < f32::EPSILON * 3.
                    // {
                    brain.t.aim.offset_angle =
                        look_at(brain.origin, unsafe { *ent.get_origin(&mut v) })
                            + brain.t.aim.spread.dequeue().unwrap_or_default();
                    // }

                    brain.next_cmd.world_view_angles =
                        natural_aim(brain.angles, brain.t.aim.offset_angle);

                    let mut v = Vector3::ZERO;
                    if let Some(point) = brain.path.get(1)
                        && weapon_state != WeaponState::Reloading // run when reloading
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
            } else if let Target::Entity(handle, false) = brain.t.shoot_target
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

            if let Target::Entity(_, false) = brain.t.shoot_target {
                brain.t.reacts_at = helper.globals.curTime + REACTON_TIME;
            }

            (Status::Success, 0.)
        }
        TargetingAction::Melee
            if let Target::Entity(handle, true) = brain.t.shoot_target
                && let Some(ent) = lookup_ent(handle, helper.sv_funcs)
                && unsafe { ent.is_titan() } == brain.is_titan =>
        {
            let mut v = Vector3::ZERO;
            let target = unsafe { *ent.get_origin(&mut v) };
            if match brain.is_titan {
                true => {
                    (brain.origin.x - target.x).powi(2) * (brain.origin.y - target.y).powi(2)
                        < 81000.
                        && (brain.origin.z - target.z).abs() < 50.
                }
                false => {
                    (brain.origin.x - target.x).powi(2) * (brain.origin.y - target.y).powi(2)
                        < 850000.
                        && (brain.origin.z - target.z).abs() < 200.
                }
            } {
                brain.m.view_lock = true;
                brain.next_cmd.buttons |= MoveAction::Melee as u32;
                brain.next_cmd.world_view_angles =
                    natural_aim(brain.angles, look_at(brain.origin, target));
                brain.t.reacts_at = helper.globals.curTime + SWITCH_TIME;
                (Status::Success, 0.)
            } else {
                (Status::Failure, 0.)
            }
        }
        TargetingAction::Melee => (Status::Failure, 0.),
        TargetingAction::OnDeathStartHate => {
            if let Some(player) = lookup_ent(bot.m_lastDeathInfo.m_hAttacker, helper.sv_funcs)
                .and_then::<&CPlayer, _>(|ent| ent.dynamic_cast())
                && !brain.looked_at_death_record
            {
                *brain.t.hates.entry(get_player_index(player)).or_default() += 1;
                brain.looked_at_death_record = true;
            }
            (Status::Success, 0.)
        }
        TargetingAction::UpdatePostTargetting => {
            if let Some(weapon) = lookup_ent(bot.m_inventory.activeWeapon, helper.sv_funcs)
                .and_then::<&CWeaponX, _>(|ent| ent.dynamic_cast())
            {
                brain.t.aim.last_weapon_state = weapon.m_weapState;
            }
            (Status::Success, 0.)
        }
    }
}

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

fn get_player_in_view(
    targeting: &Targeting,
    bot: &CPlayer,
    bot_origin: Vector3,
    helper: &CUserCmdHelper<'_>,
    base: Vec3,
) -> Option<i32> {
    make_player_iterator(bot, helper)
        .filter(|_| {
            matches!(
                targeting.mode,
                TargetingMode::Agressive | TargetingMode::PassbyAgressive
            )
        })
        .fold(None::<(Vec3, &CBaseEntity, usize, i32)>, |left, rigth| {
            if let Some(left) = left
                && (left.0.distance(base) as u32)
                    .saturating_sub(targeting.hates.get(&left.2).copied().unwrap_or_default() * 50)
                    < (rigth.0.distance(base) as u32).saturating_sub(
                        targeting.hates.get(&rigth.2).copied().unwrap_or_default() * 50,
                    )
                && let trace = trace_ray(
                    bot_origin,
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
                    bot_origin,
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
        .map(|(_, _, _, handle)| handle)
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

pub fn in_view(origin: Vector3, angles: Vector3, cmp_origin: Vector3, view_rad: f32) -> bool {
    let (origin, angles, cmp_origin) = (
        Vec3::new(origin.x, origin.y, origin.z),
        Vec3::new(angles.x, angles.y, angles.z),
        Vec3::new(cmp_origin.x, cmp_origin.y, cmp_origin.z),
    );

    (cmp_origin - origin).normalize_or_zero().dot(angles) > view_rad.cos()
}

static SPIN: AtomicU8 = AtomicU8::new(1);
fn generate_spread<const N: usize>(
    spread_buf: &mut ConstGenericRingBuffer<Vector3, N>,
    spread_rigth: bool,
    config: &TargetingConfig,
    seed: u64,
) {
    spread_buf.clear();

    let values_bottom: i32 = config.values as i32 / -3;
    let values_top: i32 = config.values as i32 * 2 / 3;

    let right_way = (values_bottom..values_top).filter(|_| spread_rigth);
    let left_way = (-values_top..-values_bottom)
        .filter(|_| !spread_rigth)
        .rev();

    let spin = SPIN
        .fetch_update(AtomicOrdering::Relaxed, AtomicOrdering::Relaxed, |x| {
            Some((x + 1) % 8)
        })
        .unwrap_or_default() as f32;

    let mut hasher = FxHasher::default();
    seed.hash(&mut hasher);
    let angle = (hasher.finish() % config.permutations) as f32 / config.permutations as f32;
    spread_buf.extend(
        right_way
            .chain(left_way)
            .map(|i| {
                let mut hasher = FxHasher::default();
                seed.hash(&mut hasher);
                config.y_fluctuation_permutations.hash(&mut hasher);
                (
                    i as f32,
                    ((hasher.finish() % config.y_fluctuation_permutations)
                        .min(config.y_fluctuation_permutations_min) as f32
                        / config.y_fluctuation_permutations as f32
                        - config.y_fluctuation_permutations as f32 / 2.)
                        * config.y_fluctuation_frac,
                )
            })
            .map(|(x, fluctuation)| {
                Vector3::new(
                    angle * x * config.values_step - fluctuation,
                    x * config.values_step * spin,
                    0.,
                )
            }),
    );
}

pub fn arc_length(left: Vec2, right: Vec2) -> f32 {
    Vec2::new(angle_diff(left.x, right.x), angle_diff(left.y, right.y)).length()
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

fn get_weapon_charge_fraction(weapon: &CWeaponX, helper: &CUserCmdHelper) -> f32 {
    let is_charge_weapon =
        i32::from_ne_bytes(array::from_fn(|i| weapon.m_modVars[0x31c..0x31c + 4][i]));

    if (is_charge_weapon > 0) && weapon.m_weapState as i32 - 5 < 2
        || weapon.m_modVars[0x334] != 0 && (0 < weapon.m_lastChargeLevel)
    {
        return (helper.globals.exactCurTime - weapon.m_chargeStartTime) / is_charge_weapon as f32;
    }
    let charge_rate = f32::from_ne_bytes(array::from_fn(|i| weapon.m_modVars[800..800 + 4][i]));
    let charge_min = 0.0;
    if 0.0 < charge_rate {
        let charge_time = (helper.globals.exactCurTime - weapon.m_chargeEndTime)
            - f32::from_ne_bytes(array::from_fn(|i| weapon.m_modVars[0x324..0x324 + 4][i])).max(0.);
        let charge_diff = weapon.m_lastChargeFrac - charge_time / charge_rate;
        if 0.0 < charge_diff {
            log::info!("charge_diff : {charge_diff}");
            charge_diff
        } else {
            log::info!("charge_min : {charge_min}");
            charge_min
        }
    } else {
        log::info!("charge_min2 : {charge_min}");
        charge_min
    }
}

pub(crate) fn classify_threats<'a>(
    _shared: &SharedBotBrain,
    behaviors: impl Iterator<Item = &'a mut BotBrain> + 'a,
) {
    let server_funcs = SERVER_FUNCTIONS.wait();
    let npcs = unsafe { get_npc_buffer(server_funcs) };

    // SAFETY: values are only read and the game thread is blocked here
    let players = (0..unsafe {
        ENGINE_FUNCTIONS
            .wait()
            .globals
            .as_ref()
            .map(|globals| globals.maxPlayers)
            .unwrap_or(32)
    })
        .flat_map(|i| unsafe {
            Some(UnsafeHandle::new(nudge_type::<&CBaseEntity>(
                (server_funcs.get_player_by_index)(i + 1).as_ref()?,
            )))
        })
        .collect::<Vec<_>>();

    for npc in npcs {
        unsafe { (server_funcs.calc_origin)(*npc, &(*npc).cast_const(), 0, 0) };
    }

    let players_slice = players.as_slice();
    std::thread::scope(move |s| {
        for brain in behaviors {
            brain.t.possible_targets.clear();
            brain.t.extra_target.take();

            // SAFETY: same as above
            let npcs = unsafe { high::UnsafeHandle::new(npcs) };
            s.spawn(move || {
                let npcs = npcs.take();

                let navmesh_ref = brain.navmesh.read();
                let NavmeshStatus::Loaded(navmesh) = &navmesh_ref.navmesh else {
                    return;
                };

                for ent in npcs
                    .iter()
                    .filter_map(|ent| unsafe { ent.as_ref() })
                    .chain(players_slice.iter().map(|h| h.get()).copied())
                    .filter(|ent| ent.m_iTeamNum != brain.team && ent.m_lifeState == 0)
                {
                    let distance = distance3(brain.abs_origin, ent.m_vecAbsOrigin);
                    let hit = navmesh.ray_cast(&RayCast3d::new(
                        Vec3A::new(brain.abs_origin.x, brain.abs_origin.y, brain.abs_origin.z),
                        Dir3A::new_and_length(Vec3A::new(
                            ent.m_vecAbsOrigin.x - brain.abs_origin.x,
                            ent.m_vecAbsOrigin.y - brain.abs_origin.y,
                            ent.m_vecAbsOrigin.z - brain.abs_origin.z,
                        ))
                        .map(|(l, _)| l)
                        .unwrap_or(Dir3A::X),
                        distance,
                    ));

                    let target = PossibleTarget {
                        handle: get_entity_handle(ent),
                        looking: Vec2::new(
                            ent.m_vecAbsOrigin.x - brain.abs_origin.x,
                            ent.m_vecAbsOrigin.y - brain.abs_origin.y,
                        )
                        .normalize()
                        .dot(Vec2::new(ent.m_angAbsRotation.x, ent.m_angAbsRotation.y).normalize()),
                        eye_contact: 0., // TODO: implement this
                        shooting: true,
                        distance,
                        is_player: DynamicCast::<CPlayer>::dynamic_cast(ent).is_some(),
                    };

                    if hit.element.is_none() {
                        brain.t.possible_targets.push(target);
                    } else if let Some(prev) = brain.t.extra_target.as_ref()
                        && (target <= *prev || target.is_player && !prev.is_player)
                        // players are more important
                        && (target.is_player || !prev.is_player)
                    {
                        brain.t.extra_target.replace(target);
                    } else if brain.t.extra_target.is_none() {
                        brain.t.extra_target.replace(target);
                    }
                }
            });
        }
    })
}
