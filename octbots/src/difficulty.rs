use arc_swap::ArcSwap;
use rrplug::{
    bindings::cvar::convar::{FCVAR_ARCHIVE, FCVAR_GAMEDLL},
    prelude::*,
};
use std::{cell::RefCell, sync::Arc};

thread_local! {
    static BOT_DIFFICULTY: RefCell<Option<ConVarStruct>> = const { RefCell::new(None) };
    static CONFIG: RefCell<Option<Arc<ArcSwap<DifficultyConfig>>>> = const { RefCell::new(None) };
}

pub type ConfigArc = Arc<arc_swap::ArcSwapAny<Arc<DifficultyConfig>>>;

pub(crate) fn default_config() -> ConfigArc {
    Arc::new(ArcSwap::from_pointee(DifficultyConfig::new(
        DiffcuiltyLevel::Normal,
    )))
}

pub(crate) fn init_difficulty(engine_token: EngineToken, config: &ConfigArc) {
    let convar_registration_info: ConVarRegister = ConVarRegister {
        callback: Some(difficulty_update_callback),
        ..ConVarRegister::mandatory(
            "bot_difficulty",
            "normal",
            FCVAR_GAMEDLL as i32 | FCVAR_ARCHIVE as i32,
            "for now only changes how the bots aim",
        )
    };
    let difficulty_convar = ConVarStruct::find_convar_by_name("bot_difficulty", engine_token)
        .or_else(|_| ConVarStruct::try_new(&convar_registration_info, engine_token))
        .expect("failed to register bot_difficulty");

    parse_difficulty(&difficulty_convar, config);

    BOT_DIFFICULTY.with(move |convar| {
        convar.borrow_mut().replace(difficulty_convar);
    });
    CONFIG.with(move |thread_config| {
        thread_config.replace(Some(Arc::clone(config)));
    })
}

#[rrplug::convar]
fn difficulty_update_callback() {
    BOT_DIFFICULTY.with(|convar| {
        CONFIG.with(|config| {
            if let (Some(convar), Some(config)) =
                (convar.borrow().as_ref(), config.borrow().as_ref())
            {
                parse_difficulty(convar, config);
            } else {
                log::error!("failed to load difficulty")
            }
        })
    });
}

fn parse_difficulty(convar: &ConVarStruct, config: &ConfigArc) {
    let new_value = convar.get_value();
    if let Some(new_difficulty) = 'scope: {
        match new_value.value.as_str() {
            "easy" => break 'scope Some(DiffcuiltyLevel::Easy),
            "normal" => break 'scope Some(DiffcuiltyLevel::Normal),
            "hard" => break 'scope Some(DiffcuiltyLevel::Hard),
            _ => {}
        }

        match new_value.value_int {
            0 => Some(DiffcuiltyLevel::Easy),
            1 => Some(DiffcuiltyLevel::Normal),
            2 => Some(DiffcuiltyLevel::Hard),
            _ => None,
        }
    } {
        config.swap(Arc::new(DifficultyConfig::new(new_difficulty)));
    } else {
        log::error!("unknown difficulty: {}", new_value.value);
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub enum DiffcuiltyLevel {
    Easy,
    #[default]
    Normal,
    Hard,
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct DifficultyConfig {
    pub targeting: TargetingConfig,
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct TargetingConfig {
    pub values: usize,
    pub values_step: f32,
    pub permutations: u64,
    pub y_fluctuation_permutations: u64,
    pub y_fluctuation_permutations_min: u64,
    pub y_fluctuation_frac: f32,
    pub view_range: f32,
    pub reaction_time: f32,
    pub target_switch_time: f32,
}

impl DifficultyConfig {
    pub fn new(difficulty: DiffcuiltyLevel) -> Self {
        Self {
            targeting: TargetingConfig::new(difficulty),
        }
    }
}

impl TargetingConfig {
    fn new(difficulty: DiffcuiltyLevel) -> Self {
        match difficulty {
            DiffcuiltyLevel::Easy => Self {
                values: 30,
                values_step: 0.2,
                permutations: 40,
                y_fluctuation_permutations: 20,
                y_fluctuation_permutations_min: 5,
                y_fluctuation_frac: 0.05,
                view_range: 240.,
                reaction_time: 0.8,
                target_switch_time: 0.5,
            },
            DiffcuiltyLevel::Normal => Self {
                values: 20,
                values_step: 0.2,
                permutations: 40,
                y_fluctuation_permutations: 10,
                y_fluctuation_permutations_min: 1,
                y_fluctuation_frac: 0.05,
                view_range: 270.,
                reaction_time: 0.4,
                target_switch_time: 0.2,
            },
            DiffcuiltyLevel::Hard => Self {
                values: 15,
                values_step: 0.2,
                permutations: 40,
                y_fluctuation_permutations: 10,
                y_fluctuation_permutations_min: 1,
                y_fluctuation_frac: 0.05,
                view_range: 270.,
                reaction_time: 0.4,
                target_switch_time: 0.2,
            },
        }
    }
}
