use rand::Rng;
use rrplug::bindings::class_types::client::SignonState;
use rrplug::prelude::*;
use rrplug::{
    bindings::convar::FCVAR_GAMEDLL,
    high::convars::{ConVarRegister, ConVarStruct},
    to_sq_string, OnceCell,
};
use std::ffi::CStr;
use std::{ops::Deref, sync::Mutex};

use crate::utils::set_c_char_array;
use crate::{
    bindings::{ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    utils::iterate_c_array_sized,
    PLUGIN,
};

use self::detour::{hook_engine, hook_server};
use self::{convars::register_required_convars, debug_commands::register_debug_concommands};

mod cmds;
mod convars;
mod debug_commands;
mod detour;
mod set_on_join;

static CLAN_TAG_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static SIMULATE_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();

#[derive(Debug)]
pub struct Bots {
    pub clang_tag: Mutex<String>,
    pub generic_bot_names: Mutex<Vec<String>>,
}

impl Plugin for Bots {
    fn new(_plugin_data: &PluginData) -> Self {
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

    fn main(&self) {}

    fn on_dll_load(&self, engine: &PluginLoadDLL, dll_ptr: &DLLPointer) {
        match dll_ptr.which_dll() {
            rrplug::mid::engine::WhichDll::Engine => hook_engine(dll_ptr.get_dll_ptr()),
            rrplug::mid::engine::WhichDll::Server => hook_server(dll_ptr.get_dll_ptr()),
            _ => {}
        }

        let engine = match engine {
            PluginLoadDLL::Engine(engine) => engine,
            _ => return,
        };

        let mut convar = ConVarStruct::try_new().unwrap();
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

        let mut simulate_convar = ConVarStruct::try_new().unwrap();
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

    let name = to_sq_string!(name);
    unsafe {
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
    }
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
fn clang_tag_changed() {
    let new_clan_tag = match CLAN_TAG_CONVAR.wait().get_value_string() {
        Ok(c) => c.to_string(),
        Err(err) => return err.log(),
    };

    let mut clan_tag = PLUGIN.wait().bots.clang_tag.lock().expect("how");
    *clan_tag = new_clan_tag;
}
