#![feature(unboxed_closures)]

use bots_convars::register_required_convars;
use debug_commands::register_debug_concommands;
use rrplug::{
    bindings::convar::FCVAR_GAMEDLL,
    wrappers::convars::{ConVarRegister, ConVarStruct},
    wrappers::northstar::{EngineLoadType, PluginData},
};
use rrplug::{prelude::*, to_sq_string, OnceCell};
use std::sync::Mutex;
use std::{ffi::c_void, mem};
use tf2dlls::SourceEngineData;

use crate::{native_types::SignonState, structs::cbaseclient::CbaseClient};

mod bots_cmds;
mod bots_convars;
mod bots_detour;
mod debug_commands;
mod native_types;
mod structs;
mod tf2dlls;

static CLAN_TAG_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static SIMULATE_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static mut TESTBOT: Option<CbaseClient> = None;

#[derive(Debug)]
pub struct BotPlugin {
    clang_tag: Mutex<String>,
    source_engine_data: Mutex<SourceEngineData>,
}

impl Plugin for BotPlugin {
    fn new() -> Self {
        Self {
            clang_tag: Mutex::new("BOT".into()),
            #[allow(invalid_value)]
            source_engine_data: Mutex::new(unsafe { mem::MaybeUninit::zeroed().assume_init() }),
        }
    }

    fn initialize(&mut self, _plugin_data: &PluginData) {}

    fn main(&self) {}

    fn on_engine_load(&self, engine: EngineLoadType) {
        let engine = match engine {
            EngineLoadType::Engine(engine) => engine,
            EngineLoadType::EngineFailed => return,
            EngineLoadType::Server => {
                std::thread::spawn(|| {
                    wait(10000);

                    PLUGIN
                        .wait()
                        .source_engine_data
                        .lock()
                        .expect("how")
                        .load_server()
                });
                return;
            }
            EngineLoadType::Client => return,
        };

        self.source_engine_data.lock().expect("how").load_engine();

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

#[rrplug::concommand]
fn spawn_fake_player(command: CCommandResult) {
    let mut source_engine_data = PLUGIN.wait().source_engine_data.lock().expect("how");

    let name = command.args.get(0).unwrap_or(&"bot".to_owned()).to_owned();
    let team = command
        .args
        .get(1)
        .map(|t| t.parse::<i32>().ok())
        .unwrap_or_else(|| Some(choose_team(&mut source_engine_data)))
        .unwrap_or_else(|| choose_team(&mut source_engine_data));
        // .clamp(32, 2);

    log::info!("bot : {name} spawned");

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

        client.peak();

        let array_addr = source_engine_data.client_array.get_inner_ptr() as usize;
        let client_addr = client.get_addr();

        let offset = client.get_addr() - array_addr;

        log::info!("offset {offset}");
        log::info!("client_addr {array_addr}");
        log::info!("array_addr {client_addr}");

        wait(1);

        let g = source_engine_data.game_clients;
        let f = source_engine_data.client_fully_connected;

        f(g, client.get_edict(), true);

        log::info!("spawned a bot : {}", client.get_name());

        _ = TESTBOT.replace(client);
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
                *((get_player_by_index(index as i32 + 1).as_ref()? as *const _ as *const c_void).offset(0x5E4)
                    as *const i32)
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

    let mut clan_tag = PLUGIN.wait().clang_tag.lock().expect("how");
    *clan_tag = new_clan_tag;
}

entry!(BotPlugin);
