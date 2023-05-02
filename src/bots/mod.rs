use rand::Rng;
use rrplug::{
    bindings::convar::FCVAR_GAMEDLL,
    to_sq_string,
    wrappers::convars::{ConVarRegister, ConVarStruct},
    wrappers::northstar::{EngineLoadType, PluginData},
};
use rrplug::{prelude::*, OnceCell};
use std::{ffi::c_void, sync::Mutex};

use crate::{
    hooks::Hooks, native_types::SignonState, structs::cbaseclient::CbaseClient,
    tf2dlls::SourceEngineData, PLUGIN,
};

use self::{convars::register_required_convars, debug_commands::register_debug_concommands};

mod cmds;
mod convars;
mod debug_commands;
mod detour;

static CLAN_TAG_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static SIMULATE_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();

#[derive(Debug)]
pub struct Bots {
    pub clang_tag: Mutex<String>,
    pub generic_bot_names: Mutex<Vec<String>>,
}

impl Plugin for Bots {
    fn new() -> Self {
        Self {
            clang_tag: Mutex::new("BOT".into()),
            generic_bot_names: Mutex::new(
                vec![
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
                    ">.<",
                    "-.-",
                    "HIIIIIII",
                    "okhuh",
                    "BOT-7274",
                    "Standby_For_BotFall",
                    "ifissmthismodded",
                    "whenmp_boxgameplay?",
                    "rust<3",
                    "hi_Fifty",
                    "yesdog",
                    "bobthebot",
                    "Ihatewarnings",
                ]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            ),
        }
    }
    fn initialize(&mut self, _plugin_data: &PluginData) {}

    fn main(&self) {}

    fn on_engine_load(&self, engine: &EngineLoadType) {
        let engine = match engine {
            EngineLoadType::Engine(engine) => engine,
            _ => return,
        };

        let convar = ConVarStruct::try_new().unwrap();
        let register_info = ConVarRegister {
            callback: Some(clang_tag_changed),
            ..ConVarRegister::mandatory(
                "bot_clang_tag",
                "BOT",
                FCVAR_GAMEDLL as i32,
                "the clan tag for the bot; use . to indicate no clan tag",
            )
        };

        convar
            .register(register_info)
            .expect("failed to register the convar");
        _ = CLAN_TAG_CONVAR.set(convar);

        let simulate_convar = ConVarStruct::try_new().unwrap();
        let register_info = ConVarRegister {
            ..ConVarRegister::mandatory(
                "bot_cmds",
                "1",
                FCVAR_GAMEDLL as i32,
                "weather bots should have cmds ran",
            )
        };

        simulate_convar
            .register(register_info)
            .expect("failed to register the convar");
        _ = SIMULATE_CONVAR.set(simulate_convar);

        register_required_convars(engine);

        _ = engine.register_concommand(
            "bot_spawn",
            spawn_fake_player,
            "spawns a bot on team 2",
            FCVAR_GAMEDLL as i32,
        );

        register_debug_concommands(engine)
    }
}

impl Hooks for Bots {
    fn hook_server(&self, dll: &crate::hooks::DllHook) {
        detour::hook_server(dll.get_ptr())
    }
    fn hook_engine(&self, dll: &crate::hooks::DllHook) {
        detour::hook_engine(dll.get_ptr())
    }
    fn hook_client(&self, dll: &crate::hooks::DllHook) {
        detour::hook_client(dll.get_ptr())
    }
}

#[rrplug::concommand]
fn spawn_fake_player(command: CCommandResult) {
    let plugin = PLUGIN.wait();
    let mut source_engine_data = plugin.source_engine_data.lock().expect("how");

    let mut rng = rand::thread_rng();
    let names = &plugin.bots.generic_bot_names.lock().expect("how");

    let name = command
        .args
        .get(0)
        .unwrap_or_else(|| {
            names
                .get(rng.gen_range(0..names.len()))
                .unwrap_or(&names[0])
        })
        .to_owned();
    let team = command
        .args
        .get(1)
        .map(|t| t.parse::<i32>().ok())
        .unwrap_or_else(|| Some(choose_team(&mut source_engine_data)))
        .unwrap_or_else(|| choose_team(&mut source_engine_data));

    let name = to_sq_string!(name);
    unsafe {
        let bot = (source_engine_data.create_fake_client)(
            source_engine_data.server,
            name.as_ptr(),
            &'\0' as *const char as *const i8,
            &'\0' as *const char as *const i8,
            team,
            0,
        );

        let client = match CbaseClient::new(bot) {
            Some(c) => c,
            None => {
                log::warn!("spawned a invalid bot");
                return;
            }
        };

        (source_engine_data.client_fully_connected)(std::ptr::null(), client.get_edict(), true);

        log::info!("spawned a bot : {}", client.get_name());
    }
}

fn choose_team(source_engine_data: &mut SourceEngineData) -> i32 {
    let client_array = &mut source_engine_data.client_array;
    let get_player_by_index = source_engine_data.player_by_index;

    let mut total_players = 0;

    let team_2_count = client_array
        .enumerate()
        .filter(|(_, c)| c.get_signon() >= SignonState::Connected)
        .inspect(|_| total_players += 1)
        .filter_map(|(index, _)| {
            Some(unsafe {
                *((get_player_by_index(index as i32 + 1).as_ref()? as *const _ as *const c_void)
                    .offset(0x5E4) as *const i32)
            })
        })
        .filter(|team| team == &2)
        .count();

    let team_3_count = total_players - team_2_count;

    if team_3_count < team_2_count {
        3
    } else {
        2
    }
}

#[rrplug::convar]
fn clang_tag_changed(convar: Option<ConVarStruct>, old_value: String, float_old_value: f32) {
    let new_clan_tag = match CLAN_TAG_CONVAR.wait().get_value_string() {
        Some(c) => c,
        None => return,
    };

    let mut clan_tag = PLUGIN.wait().bots.clang_tag.lock().expect("how");
    *clan_tag = new_clan_tag;
}
