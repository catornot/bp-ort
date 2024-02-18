use once_cell::sync::Lazy;
use rand::Rng;
use rrplug::bindings::class_types::cplayer::CPlayer;
use rrplug::mid::utils::try_cstring;
use rrplug::prelude::*;
use rrplug::{
    bindings::{
        class_types::client::{CClient, SignonState},
        cvar::convar::FCVAR_GAMEDLL,
    },
    exports::OnceCell,
    high::engine::convars::{ConVarRegister, ConVarStruct},
};
use std::{
    cell::RefCell,
    ffi::CStr,
    {ops::Deref, sync::Mutex},
};

use self::detour::{hook_engine, hook_server};
use self::{convars::register_required_convars, debug_commands::register_debug_concommands};
use crate::utils::set_c_char_array;
use crate::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::iterate_c_array_sized,
    PLUGIN,
};

mod cmds;
mod convars;
mod debug_commands;
mod detour;
// mod navmesh;
mod set_on_join;

static CLAN_TAG_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static SIMULATE_TYPE_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static DRAWWORLD_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub const DEFAULT_SIMULATE_TYPE: i32 = 5;

thread_local! {
    pub static MAX_PLAYERS: RefCell<u32> = const { RefCell::new(32) };
}
pub(super) static mut TASK_MAP: Lazy<[BotData; 64]> =
    Lazy::new(|| std::array::from_fn(|_| BotData::default()));

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum BotWeaponState {
    #[default]
    ApReady,
    ApPrepare,
    AtReady,
    AtPrepare,
    TitanReady,
    TitanPrepare,
}

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

#[derive(Debug, Default)]
pub(super) struct BotData {
    sim_type: Option<i32>,
    edict: u16,
    weapon_state: BotWeaponState,
    titan: TitanClass,
    counter: u32,
}

#[derive(Debug)]
pub struct Bots {
    pub clang_tag: Mutex<String>,
    pub generic_bot_names: Mutex<Vec<String>>,
}

impl Plugin for Bots {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new("Bots\0", "BOTS_____\0", "BOTS\0", PluginContext::all());

    fn new(_: bool) -> Self {
        register_sq_functions(bot_set_titan);

        Self {
            clang_tag: Mutex::new("BOT".into()),
            generic_bot_names: Mutex::new(
                [
                    "bot",
                    "botornot",
                    "perhaps_bot",
                    "sybotn",
                    "botsimp",
                    "1-1=-0",
                    "thx_bob",
                    "Petar_:D",
                    "HI_HOLO",
                    "ctalover",
                    "Bot3000",
                    "okhuh",
                    "BOT-7274",
                    "Standby_For_BotFall",
                    "ifissmthismodded",
                    "whenmp_boxgameplay?",
                    "rust<3",
                    "hi_Fifty",
                    "yesdogbot",
                    "bobthebot",
                    "Ihatewarnings",
                    "Trinity_Bot",
                    "bornet",
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            ),
        }
    }

    fn on_sqvm_created(&self, handle: &CSquirrelVMHandle, _token: EngineToken) {
        // DISABLED IN LIBS.RS
        match handle.get_context() {
            // doesn't seam to work anymore?
            ScriptContext::UI => {}
            _ => return,
        }

        let max_players: u32 = unsafe {
            CStr::from_ptr((ENGINE_FUNCTIONS.wait().get_current_playlist_var)(
                "max_players\0"
                    .as_ptr()
                    .cast::<i8>()
                    .as_ref()
                    .unwrap_or_else(|| &*("err\0".as_ptr() as *const i8)),
                false as i32,
            ))
            .to_string_lossy()
        }
        .parse()
        .unwrap_or_else(|_| {
            log::warn!("max_players is undefined; using default of 32");
            32
        });

        log::info!("MAX_PLAYERS is set to {max_players}");

        MAX_PLAYERS.with(|i| *i.borrow_mut() = max_players);
    }

    fn on_sqvm_destroyed(&self, handle: &CSquirrelVMHandle, _token: EngineToken) {
        if let ScriptContext::SERVER = handle.get_context() {
            let engine_functions = ENGINE_FUNCTIONS.wait();
            unsafe {
                iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
                    .filter(|client| **client.signon == SignonState::FULL && **client.fake_player)
                    .for_each(|client| {
                        (engine_functions.cclient_disconnect)(
                            (client as *const CClient).cast_mut(),
                            1,
                            "no reason\0" as *const _ as *const i8,
                        )
                    });
            }
        }
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer, token: EngineToken) {
        match dll_ptr.which_dll() {
            rrplug::mid::engine::WhichDll::Engine => hook_engine(dll_ptr.get_dll_ptr()),
            rrplug::mid::engine::WhichDll::Server => hook_server(dll_ptr.get_dll_ptr()),
            _ => {}
        }

        let Some(engine) = engine else { return };

        let convar = ConVarStruct::try_new(
            &ConVarRegister {
                callback: Some(clang_tag_changed),
                ..ConVarRegister::mandatory(
                    "bot_clang_tag",
                    "BOT",
                    FCVAR_GAMEDLL as i32,
                    "the clan tag for the bot",
                )
            },
            token,
        )
        .expect("failed to register the convar");
        _ = CLAN_TAG_CONVAR.set(convar);

        let simulate_convar = ConVarStruct::try_new(&ConVarRegister::new(
                "bot_cmds_type",
                DEFAULT_SIMULATE_TYPE.to_string(),
                FCVAR_GAMEDLL as i32,
                "the type of cmds running for bots; 0 = null, 1 = frog, 2 = following player 0, 3 = firing, 5 = firing and moving to a player, 5 = going forward",
            ), token
        ).expect("failed to register the convar");

        _ = SIMULATE_TYPE_CONVAR.set(simulate_convar);

        let draw_convar = ConVarStruct::try_new(
            &ConVarRegister::new("drawworld", "1", FCVAR_GAMEDLL as i32, ""),
            token,
        )
        .expect("failed to register the convar");

        _ = DRAWWORLD_CONVAR.set(draw_convar);

        register_required_convars(engine, token);

        _ = engine.register_concommand_with_completion(
            "bot_spawn",
            spawn_fake_player,
            "spawns a bot",
            FCVAR_GAMEDLL as i32,
            spawn_fake_player_completion,
            token,
        );

        register_debug_concommands(engine, token);
    }
}

#[rrplug::concommand]
fn spawn_fake_player(command: CCommandResult) {
    let plugin = PLUGIN.wait();
    let engine_funcs = ENGINE_FUNCTIONS.wait();
    let mut rng = rand::thread_rng();
    let names = &plugin.bots.generic_bot_names.lock().expect("how");

    let name = command
        .get_args()
        .get(0)
        .unwrap_or_else(|| {
            names
                .get(rng.gen_range(0..names.len()))
                .unwrap_or(&names[0])
        })
        .to_owned();
    let team = command
        .get_args()
        .get(1)
        .map(|t| t.parse::<i32>().ok())
        .unwrap_or_else(|| Some(choose_team()))
        .unwrap_or_else(choose_team);

    let sim_type = command
        .get_args()
        .get(2)
        .and_then(|t| t.parse::<i32>().ok()); // doesn't work?

    let name = try_cstring(&name).unwrap_or_default();
    unsafe {
        let players = iterate_c_array_sized::<_, 32>(engine_funcs.client_array.into())
            .filter(|c| c.signon.get_inner() >= &SignonState::CONNECTED)
            .count() as u32;
        let max_players = MAX_PLAYERS.with(|i| *i.borrow());
        if players >= max_players {
            log::warn!(
                "max players({}) reached({}) can't add more",
                max_players,
                players
            );
            return;
        }

        let bot = (engine_funcs.create_fake_client)(
            engine_funcs.server,
            name.as_ptr(),
            &'\0' as *const char as *const i8,
            &'\0' as *const char as *const i8,
            team,
            0,
        );

        let client = match bot.cast_mut().as_mut() {
            Some(c) => c,
            None => {
                log::warn!("spawned a invalid bot");
                return;
            }
        };

        (SERVER_FUNCTIONS.wait().client_fully_connected)(std::ptr::null(), **client.edict, true);

        log::info!(
            "spawned a bot : {}",
            CStr::from_ptr(client.name.as_ref() as *const [i8] as *const i8).to_string_lossy()
        );

        set_c_char_array(
            &mut client.clan_tag,
            &PLUGIN.wait().bots.clang_tag.lock().expect("how"),
        );

        *TASK_MAP
            .get_mut(**client.edict as usize)
            .expect("tried to get an invalid edict") = BotData {
            sim_type,
            ..Default::default()
        };
    }
}

#[rrplug::completion]
fn spawn_fake_player_completion(current: CurrentCommand, suggestions: CommandCompletion) -> i32 {
    let Some((name, team)) = current.partial.split_once(' ') else {
        _ = suggestions.push(&format!("{} {}", current.cmd, current.partial));
        _ = suggestions.push(&format!("{} bot_name", current.cmd));
        return suggestions.commands_used();
    };

    let Some((prev_team, _cmd_type)) = team.split_once(' ') else {
        if team.starts_with('i') {
            _ = suggestions.push(&format!("{} {} 2", current.cmd, name));
        } else if team.starts_with('m') {
            _ = suggestions.push(&format!("{} {} 3", current.cmd, name));
        } else {
            _ = suggestions.push(&format!("{} {} 2", current.cmd, name));
            _ = suggestions.push(&format!("{} {} 3", current.cmd, name));
        }

        return suggestions.commands_used();
    };

    (0..=6).for_each(|i| {
        _ = suggestions.push(&format!("{} {} {} {}", current.cmd, name, prev_team, i))
    });

    suggestions.commands_used()
}
fn choose_team() -> i32 {
    let server_functions = SERVER_FUNCTIONS.wait();

    let mut total_players = 0;

    let team_2_count =
        unsafe { iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()) }
            .enumerate()
            .filter(|(_, c)| unsafe { c.signon.get_inner() } >= &SignonState::CONNECTED)
            .inspect(|_| total_players += 1)
            .filter_map::<i32, _>(|(index, _)| {
                Some(*unsafe {
                    (server_functions.get_player_by_index)(index as i32 + 1)
                        .as_ref()?
                        .team
                        .deref()
                        .deref()
                })
            })
            .filter(|team| *team == 2)
            .count();

    let team_3_count = total_players - team_2_count;

    if team_3_count < team_2_count {
        3
    } else {
        2
    }
}

#[rrplug::convar]
fn clang_tag_changed() {
    let new_clan_tag = match CLAN_TAG_CONVAR.wait().get_value_str() {
        Ok(c) => c.to_string(),
        Err(err) => return err.log(),
    };

    let mut clan_tag = PLUGIN.wait().bots.clang_tag.lock().expect("how");
    *clan_tag = new_clan_tag;
}

#[rrplug::sqfunction(VM = "Server", ExportName = "BotSetTitan")]
fn bot_set_titan(bot: Option<&mut CPlayer>, titan: String) -> Option<()> {
    let bot_handle = unsafe {
        TASK_MAP.get_mut(
            ENGINE_FUNCTIONS
                .wait()
                .client_array
                .add((bot?.player_index.copy_inner() - 1) as usize)
                .as_ref()?
                .edict
                .copy_inner() as usize,
        )
    }?;

    bot_handle.titan = match titan.as_str().trim() {
        "titan_stryder_arc" | "titan_stryder_leadwall" | "titan_stryder_ronin_prime" => {
            TitanClass::Ronin
        }
        "titan_stryder_sniper" | "titan_stryder_northstar_prime" => TitanClass::Northstar,
        "titan_atlas_tracker" | "titan_atlas_tone_prime" => TitanClass::Tone,
        "titan_atlas_vanguard" => TitanClass::Monarch,
        "titan_atlas_stickybomb" | "titan_atlas_ion_prime" => TitanClass::Ion,
        "titan_ogre_meteor" | "titan_ogre_scorch_prime" => TitanClass::Scorch,
        "titan_ogre_minigun" | "titan_ogre_legion_prime" => TitanClass::Legion,
        _ => TitanClass::Ion,
    };

    None
}
