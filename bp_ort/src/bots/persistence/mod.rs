use std::collections::HashMap;

use once_cell::sync::OnceCell;
use rrplug::bindings::server::client::CClient;
use shared::persistence::ClientPersistence;

use crate::interfaces::ENGINE_INTERFACES;

mod enums;

mod asillybot;
mod botornot;

macro_rules! pdata_struct {
    (struct $name:ident { $($fname:ident : $ftype:ty),*, }) => {
        #[allow(non_snake_case)]
        #[derive(Debug, Clone, Copy)]
        pub struct $name {
            $($fname : $ftype),*
        }

        impl $name {
            fn into_hash_map(self) -> HashMap<String, PDataValue> {
                HashMap::from_iter([$((stringify!($fname).to_string(), PDataValue::from(self.$fname))),*].into_iter())
            }
        }
    }
}

pdata_struct! {
    struct PilotLoadout {
        suit: &'static str,
        race: &'static str,
        execution: &'static str,
        primary: &'static str,
        primaryAttachment: &'static str,
        primaryMod1: &'static str,
        primaryMod2: &'static str,
        primaryMod3: &'static str,
        secondary: &'static str,
        secondaryMod1: &'static str,
        secondaryMod2: &'static str,
        secondaryMod3: &'static str,
        weapon3: &'static str,
        weapon3Mod1: &'static str,
        weapon3Mod2: &'static str,
        weapon3Mod3: &'static str,
        ordnance: &'static str,
        passive1: &'static str,
        passive2: &'static str,
        skinIndex: i32,
        camoIndex: i32,
        primarySkinIndex: i32,
        primaryCamoIndex: i32,
        secondarySkinIndex: i32,
        secondaryCamoIndex: i32,
        weapon3SkinIndex: i32,
        weapon3CamoIndex: i32,
    }
}

pdata_struct! {
    struct TitanLoadout {
      titanClass:&'static str,
      primaryMod:&'static str,
      special:&'static str,
      antirodeo:&'static str,
      passive1:&'static str,
      passive2:&'static str,
      passive3:&'static str,
      passive4:&'static str,
      passive5:&'static str,
      passive6:&'static str,
      titanExecution:&'static str,
      isPrime: &'static str,
      skinIndex: i32,
      camoIndex: i32,
      decalIndex: i32,
      primarySkinIndex: i32,
      primaryCamoIndex: i32,
      primeSkinIndex: i32,
      primeCamoIndex: i32,
      primeDecalIndex: i32,
      showArmBadge: i32,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PDataValue {
    String(&'static str),
    Int(i32),
}

#[derive(Debug)]
pub struct BotLoadouts {
    pilot_loadouts: HashMap<String, Vec<HashMap<String, PDataValue>>>,
    titan_loadouts: HashMap<String, Vec<HashMap<String, PDataValue>>>,
    persistence: OnceCell<ClientPersistence>,
}

impl From<&'static str> for PDataValue {
    fn from(value: &'static str) -> Self {
        PDataValue::String(value)
    }
}

impl From<i32> for PDataValue {
    fn from(value: i32) -> Self {
        PDataValue::Int(value)
    }
}

#[allow(clippy::type_complexity)]
impl BotLoadouts {
    pub fn new() -> Self {
        let generators: [(
            &'static str,
            fn() -> Vec<PilotLoadout>,
            fn() -> Vec<TitanLoadout>,
        ); 2] = [
            ("botornot", botornot::pilot, botornot::titan),
            ("ASillyBot", asillybot::pilot, asillybot::titan),
        ];

        Self {
            pilot_loadouts: generators
                .iter()
                .map(|(name, pilot, _)| {
                    (
                        name.to_string(),
                        pilot()
                            .into_iter()
                            .cycle()
                            .take(10)
                            .map(PilotLoadout::into_hash_map)
                            .collect(),
                    )
                })
                .collect(),
            titan_loadouts: generators
                .iter()
                .map(|(name, _, titan)| {
                    (
                        name.to_string(),
                        titan()
                            .into_iter()
                            .cycle()
                            .take(7)
                            .map(TitanLoadout::into_hash_map)
                            .collect(),
                    )
                })
                .collect(),
            persistence: OnceCell::new(),
        }
    }

    pub fn apply(&self, bot: &CClient, name: &str) {
        let persistence = self
            .persistence
            .get_or_init(|| ClientPersistence::new(ENGINE_INTERFACES.wait().engine_server, true));
        let name = name.to_string();

        for (index, loadout) in self
            .pilot_loadouts
            .get(&name)
            .into_iter()
            .flatten()
            .enumerate()
        {
            // log::info!("found loadout pilot for {name} {index}");
            for (property, value) in loadout {
                apply_loadout(
                    persistence,
                    &name,
                    bot,
                    "pilot",
                    index,
                    property.as_str(),
                    *value,
                );
            }
        }

        for (index, loadout) in self
            .titan_loadouts
            .get(&name)
            .into_iter()
            .flatten()
            .enumerate()
        {
            // log::info!("found loadout titan for {name} {index}");
            for (property, value) in loadout {
                apply_loadout(
                    persistence,
                    &name,
                    bot,
                    "titan",
                    index,
                    property.as_str(),
                    *value,
                );
            }
        }
    }
}

impl Default for BotLoadouts {
    fn default() -> Self {
        Self::new()
    }
}

pub fn apply_loadout(
    persistence: &ClientPersistence,
    name: &str,
    bot: &CClient,
    ty: &str,
    index: usize,
    property: &str,
    value: PDataValue,
) {
    let err = match value {
        PDataValue::String(value) => {
            let keep =
                persistence.set_player_loadout_persistence_string(bot, ty, index, property, value);
            if keep.is_err()
                && let Some(enum_index) = enums::string_to_enum(property, value)
            {
                persistence.set_player_loadout_persistence_int(bot, ty, index, property, enum_index)
            } else {
                keep
            }
        }
        PDataValue::Int(value) => {
            persistence.set_player_loadout_persistence_int(bot, ty, index, property, value)
        }
    };

    if let Err(err) = err {
        log::warn!("failed to set {ty} {property} at {index} for {name} as {value:?} : {err}");
    }
}
