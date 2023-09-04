use rrplug::bindings::cvar::convar::FCVAR_GAMEDLL;
use rrplug::prelude::*;

macro_rules! bot_convar {
    ($engine: expr,$name: expr, $default_text: expr, $flag: expr, $help_text: expr) => {
        $engine
            .register_convar($name.to_string(), "", $help_text, $flag as i32)
            .expect("failed to register convar")
    };
}

pub fn register_required_convars(engine: &EngineData) {
    bot_convar!(
        engine,
        "bot_pilot_settings",
        "",
        FCVAR_GAMEDLL,
        "force pilot playersettings for bots"
    );
    bot_convar!(
        engine,
        "bot_force_pilot_primary",
        "",
        FCVAR_GAMEDLL,
        "force pilot primary weapon for bots"
    );
    bot_convar!(
        engine,
        "bot_force_pilot_secondary",
        "",
        FCVAR_GAMEDLL,
        "force pilot secondary weapon for bots"
    );
    bot_convar!(
        engine,
        "bot_force_pilot_weapon3",
        "",
        FCVAR_GAMEDLL,
        "force pilot 3rd weapon for bots"
    );
    bot_convar!(
        engine,
        "bot_force_pilot_ordnance",
        "",
        FCVAR_GAMEDLL,
        "force pilot ordnance for bots"
    );
    bot_convar!(
        engine,
        "bot_force_pilot_ability",
        "",
        FCVAR_GAMEDLL,
        "force pilot ability for bots"
    );

    bot_convar!(
        engine,
        "bot_titan_settings",
        "",
        FCVAR_GAMEDLL,
        "force titan playersettings for bots"
    );
    bot_convar!(
        engine,
        "bot_force_titan_ordnance",
        "",
        FCVAR_GAMEDLL,
        "force titan ordnance for bots"
    );
    bot_convar!(
        engine,
        "bot_force_titan_ability",
        "",
        FCVAR_GAMEDLL,
        "force titan ability for bots"
    );
}
