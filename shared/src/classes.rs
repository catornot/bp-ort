use rrplug::{
    bindings::server::{cplayer::CPlayer, ctitan_soul::CTitanSoul},
    prelude::DynamicCast,
};

use crate::{
    bindings::SERVER_FUNCTIONS,
    utils::{get_c_char_array, lookup_ent},
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum TitanClass {
    #[default]
    Ion,
    Northstar,
    Scorch,
    Ronin,
    Tone,
    Legion,
    Monarch,
}

pub fn get_player_titan_class(player: &CPlayer) -> Option<TitanClass> {
    let class = match get_player_titan_class_str(player)?.trim() {
        "titan_stryder_arc" | "titan_stryder_leadwall" | "titan_stryder_ronin_prime" => {
            TitanClass::Ronin
        }
        "titan_stryder_sniper" | "titan_stryder_northstar_prime" => TitanClass::Northstar,
        "titan_atlas_tracker" | "titan_atlas_tone_prime" => TitanClass::Tone,
        "titan_atlas_vanguard" => TitanClass::Monarch,
        "titan_atlas_stickybomb" | "titan_atlas_ion_prime" => TitanClass::Ion,
        "titan_ogre_meteor" | "titan_ogre_scorch_prime" => TitanClass::Scorch,
        "titan_ogre_minigun" | "titan_ogre_legion_prime" => TitanClass::Legion,
        _ => None?,
    };

    Some(class)
}

pub fn get_player_titan_class_str(player: &CPlayer) -> Option<&'static str> {
    lookup_ent(player.m_titanSoul, SERVER_FUNCTIONS.wait())
        .and_then::<&CTitanSoul, _>(|soul| soul.dynamic_cast())
        .and_then(|soul| get_player_class_by_index(soul.m_playerSettingsNum as usize))
}

pub fn get_player_class_by_index(index: usize) -> Option<&'static str> {
    unsafe {
        SERVER_FUNCTIONS
            .wait()
            .loaded_player_classes
            .add(index)
            .as_ref()
            .and_then(|class| get_c_char_array(&class.class_name))
    }
}
