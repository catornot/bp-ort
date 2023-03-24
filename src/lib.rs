#![feature(unboxed_closures)]

use bots_convars::register_required_convars;
use debug_commands::register_debug_concommands;
use rrplug::{
    bindings::convar::FCVAR_GAMEDLL,
    wrappers::convars::{ConVarRegister, ConVarStruct},
    wrappers::northstar::{EngineLoadType, PluginData},
};
use rrplug::{prelude::*, to_sq_string, OnceCell};
use std::mem;
use std::sync::Mutex;
use tf2dlls::SourceEngineData;

use crate::structs::cbaseclient::CbaseClient;

mod bots_cmds;
mod bots_convars;
mod bots_detour;
mod debug_commands;
mod native_types;
mod structs;
mod tf2dlls;

static CLAN_TAG_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static SIMULATE_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();

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
    let name = command.args.get(0).unwrap_or(&"bot".to_owned()).to_owned();
    let team = command
        .args
        .get(1)
        .unwrap_or(&"2".to_owned())
        .parse::<i32>()
        .unwrap_or(2);

    let source_engine_data = PLUGIN.wait().source_engine_data.lock().expect("how");

    log::info!("bot : {name} spawned");

    let name = to_sq_string!(name);
    unsafe {
        let bot = (source_engine_data.create_fake_client)(
            source_engine_data.server,
            name.as_ptr(),
            "\0".as_ptr() as *const i8,
            "\0".as_ptr() as *const i8,
            team,
        );

        if bot.is_null() {
            log::warn!("spawned a invalid bot");
            return;
        }

        let client = CbaseClient::new(bot);

        client.peak();

        let array_addr = source_engine_data.client_array.get_inner_ptr() as usize;
        let client_addr = client.get_addr();

        let offset = client.get_addr() - array_addr;

        log::info!("offset {offset}");
        log::info!("client_addr {array_addr}");
        log::info!("array_addr {client_addr}");

        (source_engine_data.client_fully_connected)(
            source_engine_data.game_clients,
            client.get_edict(),
            false,
        );
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
