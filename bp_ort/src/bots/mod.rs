use chrono::Datelike;
use mid::utils::from_char_ptr;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use rand::Rng;
use rrplug::{
    bindings::{
        class_types::{
            client::{CClient, SignonState},
            cplayer::CPlayer,
        },
        cvar::convar::FCVAR_GAMEDLL,
    },
    exports::OnceCell,
    mid::{squirrel::SQVM_SERVER, utils::try_cstring},
    prelude::*,
};
use shared::bindings::HostState;
use simple_bot_manager::ManagerData;
use std::{
    cell::RefCell,
    collections::HashMap,
    ops::Not,
    sync::atomic::{AtomicI32, AtomicU32, Ordering},
};

use crate::{
    bindings::{EngineFunctions, ServerFunctions, ENGINE_FUNCTIONS, SERVER_FUNCTIONS},
    bots::{
        convars::register_required_convars,
        debug_commands::register_debug_concommands,
        detour::{hook_engine, hook_server},
    },
    interfaces::ENGINE_INTERFACES,
    navmesh::{navigation::Navigation, Hull},
    utils::{get_c_char_array, iterate_c_array_sized},
    PLUGIN,
};

mod cmds;
mod cmds_exec;
mod cmds_helper;
mod cmds_interface;
mod cmds_utils;
mod convars;
mod debug_commands;
mod detour;
mod netvars;
mod set_on_join;
mod simple_bot_manager;

pub const DEFAULT_SIMULATE_TYPE: i32 = 6;

static BASE_AIM_PENALTY: OnceCell<ConVarStruct> = OnceCell::new();
static CLAN_TAG_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static SIMULATE_TYPE_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();
pub static UWUFY_CONVAR: OnceCell<ConVarStruct> = OnceCell::new();

pub static AIM_PENALTY_VALUE: AtomicI32 = AtomicI32::new(100);

pub(super) static BOT_DATA_MAP: EngineGlobal<RefCell<Lazy<[BotData; 64]>>> =
    EngineGlobal::new(RefCell::new(Lazy::new(|| {
        std::array::from_fn(|_| BotData::default())
    })));
pub(super) static SHARED_BOT_DATA: EngineGlobal<Lazy<RefCell<BotShared>>> =
    EngineGlobal::new(Lazy::new(|| RefCell::new(BotShared::default())));

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
    titan: TitanClass,
    counter: u32,
    nav_query: Option<Navigation>,
    next_target_pos: Vector3,
    last_time_node_reached: f32,
    jump_delay: f32,
    jump_delay_obstacle: f32,
    jump_hold: u32,
    last_bad_path: f32,
    last_target_index: i32,
    target_pos: Vector3,
    last_shot: f32,
    should_recaculate_path: bool,
    next_check: f32,
    has_started_to_slide_hop: bool,
    approach_range: Option<f32>,
    spread: [Vector3; 20],
    spread_offset: usize,
    patrol_target: Option<Vector3>,
    last_moved_from_cap: f32,
}

#[derive(Debug)]
pub(super) struct BotShared {
    reserved_targets: [(f32, u32); 64],
    claimed_hardpoints: HashMap<Vector3, usize>,
}

impl Default for BotShared {
    fn default() -> Self {
        Self {
            reserved_targets: std::array::from_fn(|_| Default::default()),
            claimed_hardpoints: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct Bots {
    pub clang_tag: Mutex<String>,
    pub generic_bot_names: Mutex<Vec<String>>,
    pub next_bot_names: Mutex<Vec<String>>,
    pub max_players: AtomicU32,
    pub max_teams: AtomicU32,
    pub player_names: Mutex<HashMap<[i8; 32], (String, String)>>,
    pub manager_data: Mutex<simple_bot_manager::ManagerData>,
    pub external_simulations: &'static cmds_interface::ExternalSimulations,
}

impl Plugin for Bots {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"Bots", c"BOTS_____", c"BOTS", PluginContext::all());

    fn new(_: bool) -> Self {
        register_sq_functions(bot_set_titan);
        register_sq_functions(bot_set_target_pos);
        register_sq_functions(bot_set_sim_type);
        register_sq_functions(bot_spawn);
        register_sq_functions(remember_name_override);
        register_sq_functions(remember_name_override_uid);
        simple_bot_manager::register_manager_sq_functions();
        let external_simulations = unsafe {
            register_interface(
                "ExternalSimulation001",
                cmds_interface::ExternalSimulations::new(),
            )
        };

        let mut bot_names = [
            "FiveBots",
            "bot",
            "botornot",
            "perhaps_bot",
            "synbotli",
            "thx_bob",
            "Botar",
            "hOlOB0t",
            "ctalover",
            "Bot3000",
            "okhuh",
            "BOT-7274",
            "Standby_For_BotFall",
            "rust<3",
            "FiftyBots",
            "yesbot",
            "bobthebot",
            "Ihatewarnings",
            "Triboty",
            "bornet",
            "4b",
            "BlueBot",
            "Bobby_McBotFace",
            "sb0tdge",
            "JustANormalBot",
            "Bot0358",
            "Bot9182",
            "ABotPlaysGames",
            "GeckoBot",
            "FrontierBotter",
            "UniBot",
            "Bolf109909",
            "ASillyBot",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let time = chrono::Utc::now();

        if time.month() == 12 && time.day() > 15 {
            bot_names = [
                "santa",
                "5minutes",
                "christmas_in_a_week",
                "overworked_elf",
                "santa_missile",
                "sled",
                "skates",
                "christmas_tree",
            ]
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<String>>();

            bot_names.push(format!("waiting_for_{}", time.year() + 1));
        } else if time.month() == 1 && time.day() < 5 {
            bot_names.push(format!("it_s_{}_my_dudes", time.year()));
        }

        Self {
            clang_tag: Mutex::new("BOT".into()),
            next_bot_names: Mutex::new(bot_names.clone()),
            generic_bot_names: Mutex::new(bot_names),
            max_players: AtomicU32::new(32),
            max_teams: AtomicU32::new(2),
            player_names: Mutex::new(HashMap::new()),
            manager_data: Mutex::new(ManagerData::default()),
            external_simulations,
        }
    }

    fn on_sqvm_created(&self, handle: &CSquirrelVMHandle, token: EngineToken) {
        match handle.get_context() {
            ScriptContext::SERVER => {}
            _ => return,
        }

        cmds::reset_on_new_game();

        SHARED_BOT_DATA.get(token).replace(BotShared::default());

        self.next_bot_names.lock().clear();

        let max_players: u32 = unsafe {
            from_char_ptr((ENGINE_FUNCTIONS.wait().get_current_playlist_var)(
                c"max_players"
                    .as_ptr()
                    .cast::<i8>()
                    .as_ref()
                    .unwrap_or_else(|| &*c"err".as_ptr()),
                false as i32,
            ))
        }
        .parse()
        .unwrap_or_else(|_| {
            log::warn!("max_players is undefined; using default of 32");
            32
        });

        let max_teams: u32 = unsafe {
            from_char_ptr((ENGINE_FUNCTIONS.wait().get_current_playlist_var)(
                c"max_teams"
                    .as_ptr()
                    .cast::<i8>()
                    .as_ref()
                    .unwrap_or_else(|| &*c"err".as_ptr()),
                false as i32,
            ))
        }
        .parse()
        .unwrap_or_else(|_| {
            log::warn!("max_teams is undefined; using default of 2");
            2
        });

        log::info!("MAX_PLAYERS is set to {max_players}");
        log::info!("MAX_TEAMS is set to {max_teams}");

        self.max_players.store(max_players, Ordering::Release);
        self.max_teams.store(max_teams, Ordering::Release);
    }

    fn on_sqvm_destroyed(&self, handle: &CSquirrelVMHandle, _token: EngineToken) {
        if let ScriptContext::SERVER = handle.get_context() {
            let engine_functions = ENGINE_FUNCTIONS.wait();
            unsafe {
                iterate_c_array_sized::<_, 32>(engine_functions.client_array.into())
                    .filter(|client| {
                        client.m_nSignonState == SignonState::FULL && client.m_bFakePlayer
                    })
                    .for_each(|client| {
                        (engine_functions.cclient_disconnect)(
                            (client as *const CClient).cast_mut(),
                            1,
                            c"no reason".as_ptr().cast(),
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

        let convar = ConVarStruct::try_new(
            &ConVarRegister {
                callback: Some(aim_penalty_changed),
                ..ConVarRegister::mandatory(
                    "bot_aim_penalty_speed",
                    "100",
                    FCVAR_GAMEDLL as i32,
                    "the speed at which bots start having random aim applied",
                )
            },
            token,
        )
        .expect("failed to register the convar");
        _ = BASE_AIM_PENALTY.set(convar);

        let simulate_convar = ConVarStruct::try_new(&ConVarRegister::new(
                "bot_cmds_type",
                DEFAULT_SIMULATE_TYPE.to_string(),
                FCVAR_GAMEDLL as i32,
                "the type of cmds running for bots; 0 = null, 1 = frog, 2 = following player 0, 3 = firing, 5 = firing and moving to a player, 5 = going forward",
            ), token
        ).expect("failed to register the convar");

        _ = SIMULATE_TYPE_CONVAR.set(simulate_convar);

        _ = UWUFY_CONVAR.set(
            ConVarStruct::try_new(
                &ConVarRegister::new(
                    "bot_uwufy",
                    "0",
                    FCVAR_GAMEDLL as i32,
                    "decides weather connecting player should haev their name uwufyied",
                ),
                token,
            )
            .expect("failed to register the bot_uwufy convar"),
        );

        _ = engine.register_concommand_with_completion(
            "bot_spawn",
            spawn_fake_player_command,
            "spawns a bot",
            FCVAR_GAMEDLL as i32,
            spawn_fake_player_completion,
            token,
        );

        register_required_convars(engine, token);
        register_debug_concommands(engine, token);
        simple_bot_manager::register_manager_vars(engine, token);
    }

    fn runframe(&self, engine_token: EngineToken) {
        if self.manager_data.lock().enabled
            && SQVM_SERVER
                .get(engine_token)
                .try_borrow()
                .ok()
                .filter(|sqvm| sqvm.is_some())
                .is_some()
            && unsafe {
                ENGINE_FUNCTIONS
                    .wait()
                    .host_state
                    .as_ref()
                    .map(|state| state.current_state == HostState::Run)
                    .unwrap_or_default()
            }
            && let Err(err) = simple_bot_manager::check_player_amount(self, engine_token)
        {
            log::error!("bot manager: {err}");
        }
    }
}

fn spawn_fake_player(
    name: String,
    team: i32,
    sim_type: Option<i32>,
    server_funcs: &ServerFunctions,
    engine_funcs: &EngineFunctions,
    token: EngineToken,
) -> Option<i32> {
    let plugin = PLUGIN.wait();
    let engine_server = ENGINE_INTERFACES.wait().engine_server;
    let players = unsafe { iterate_c_array_sized::<_, 32>(engine_funcs.client_array.into()) }
        .filter(|c| c.m_nSignonState >= SignonState::CONNECTED)
        .count() as u32;
    let max_players = plugin.bots.max_players.load(Ordering::Acquire);
    if players >= max_players {
        log::warn!("max players({max_players}) reached({players}) can't add more");
        return None;
    }

    unsafe { engine_server.LockNetworkStringTables(true) };

    let name = try_cstring(&name).unwrap_or_default();
    let bot = unsafe {
        (engine_funcs.create_fake_client)(
            engine_funcs.server.cast_const(),
            name.as_ptr(),
            &'\0' as *const char as *const i8,
            &'\0' as *const char as *const i8,
            team,
            0,
        )
    };

    let client = match unsafe { bot.cast_mut().as_mut() } {
        Some(c) => c,
        None => {
            log::warn!("spawned a invalid bot");
            return None;
        }
    };

    let handle = client.m_nHandle;
    unsafe { (server_funcs.client_fully_connected)(std::ptr::null(), handle, true) };

    unsafe { engine_server.LockNetworkStringTables(false) };

    log::info!(
        "spawned a bot : {} with handle {handle} {}",
        get_c_char_array(&client.m_szServerName).unwrap_or("UNK"),
        unsafe {
            from_char_ptr((server_funcs.get_entity_name)((server_funcs
                .get_player_by_index)(
                handle as i32 + 1
            )))
        }
    );

    let mut shared_data = SHARED_BOT_DATA.get(token).borrow_mut();
    if let Some(hardpoint) = shared_data
        .claimed_hardpoints
        .iter()
        .find(|(_, index)| **index == handle as usize)
        .map(|(v, _)| v)
        .cloned()
    {
        _ = shared_data
            .claimed_hardpoints
            .extract_if(|v, _| *v == hardpoint);
    }

    *BOT_DATA_MAP
        .get(token)
        .borrow_mut()
        .get_mut(handle as usize)
        .expect("tried to get an invalid edict") = BotData {
        sim_type,
        nav_query: Navigation::new(Hull::Human),
        ..Default::default()
    };

    plugin
        .bots
        .external_simulations
        .simulations
        .read()
        .values()
        .filter_map(|sim| sim.init_func.as_ref())
        .for_each(|init_func| (init_func)(handle, client));

    Some(handle as i32)
}

fn get_bot_name() -> String {
    let mut next_names = PLUGIN.wait().bots.next_bot_names.lock();
    let bot_names = PLUGIN.wait().bots.generic_bot_names.lock();
    let mut rng = rand::thread_rng();

    if next_names.is_empty() {
        next_names.extend_from_slice(bot_names.as_slice());
    };

    match next_names
        .is_empty()
        .not()
        .then(|| rng.gen_range(0..next_names.len()))
        .map(|i| next_names.swap_remove(i))
    {
        Some(name) => name,
        None => {
            next_names.extend_from_slice(bot_names.as_slice());
            next_names.pop().unwrap_or_else(|| "error".to_lowercase())
        }
    }
}

#[rrplug::concommand]
fn spawn_fake_player_command(command: CCommandResult) {
    let engine_funcs = ENGINE_FUNCTIONS.wait();

    let name = command
        .get_args()
        .first()
        .cloned()
        .unwrap_or_else(get_bot_name)
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
        .and_then(|t| t.parse::<i32>().ok());

    _ = spawn_fake_player(
        name,
        team,
        sim_type,
        SERVER_FUNCTIONS.wait(),
        engine_funcs,
        engine_token,
    );
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

    (0..=12).for_each(|i| {
        _ = suggestions.push(&format!("{} {} {} {}", current.cmd, name, prev_team, i))
    });

    suggestions.commands_used()
}

fn choose_team_normal() -> i32 {
    let server_functions = SERVER_FUNCTIONS.wait();

    let mut total_players = 0;

    let team_2_count =
        unsafe { iterate_c_array_sized::<_, 32>(ENGINE_FUNCTIONS.wait().client_array.into()) }
            .enumerate()
            .filter(|(_, c)| c.m_nSignonState >= SignonState::CONNECTED)
            .inspect(|_| total_players += 1)
            .filter_map::<i32, _>(|(index, _)| {
                Some(unsafe {
                    (server_functions.get_player_by_index)(index as i32 + 1)
                        .as_ref()?
                        .m_iTeamNum
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

fn choose_team_ffa(max_teams: u32) -> i32 {
    const TEAM_OFFSET: u32 = 2;
    let server_functions = SERVER_FUNCTIONS.wait();
    let teams = Vec::from_iter((TEAM_OFFSET..=max_teams + TEAM_OFFSET).map(|_| 0));

    (1..=PLUGIN.wait().bots.max_players.load(Ordering::Acquire) as i32)
        .filter_map(|i| unsafe { (server_functions.get_player_by_index)(i).as_ref() })
        .filter_map(|player| (player.m_iTeamNum as u32).checked_sub(TEAM_OFFSET + 1)) // remove any bad teams
        .fold(teams, |mut map, team| {
            if let Some(team_slot) = map.get_mut(team as usize) {
                *team_slot += 1;
            } else {
                map[0] += 1
            }

            map
        })
        .into_iter()
        .enumerate()
        .map(|(team, amount)| (team as u32 + TEAM_OFFSET + 1, amount))
        .filter(|(team, _)| *team >= 2)
        .reduce(|left, rigth| if left.1 < rigth.1 { left } else { rigth })
        .map(|(team, _)| team as i32)
        .unwrap_or(2)
}

fn choose_team() -> i32 {
    let max_teams = PLUGIN.wait().bots.max_teams.load(Ordering::Acquire);

    if max_teams > 2 {
        let team = choose_team_ffa(max_teams);
        log::info!("chosen {team}");
        team
    } else {
        choose_team_normal()
    }
}

#[rrplug::convar]
fn clang_tag_changed() {
    let new_clan_tag = match CLAN_TAG_CONVAR.wait().get_value_str() {
        Ok(c) => c.to_string(),
        Err(err) => return err.log(),
    };

    let mut clan_tag = PLUGIN.wait().bots.clang_tag.lock();
    *clan_tag = new_clan_tag;
}
#[rrplug::convar]
fn aim_penalty_changed() -> Option<()> {
    AIM_PENALTY_VALUE.store(
        BASE_AIM_PENALTY
            .get()
            .map(|convar| convar.get_value_i32())?,
        Ordering::Relaxed,
    );

    None
}

#[rrplug::sqfunction(VM = "Server", ExportName = "BotSetTitan")]
fn bot_set_titan(bot: Option<&mut CPlayer>, titan: String) -> Option<()> {
    let mut data_maps = BOT_DATA_MAP.get(engine_token).try_borrow_mut().ok()?;
    let bot_data = data_maps.as_mut().get_mut(bot?.pl.index as usize)?; // index and edict should be the same

    bot_data.titan = match titan.as_str().trim() {
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

#[rrplug::sqfunction(VM = "Server", ExportName = "BotSetTargetPos")]
fn bot_set_target_pos(bot: Option<&mut CPlayer>, target: Vector3) -> Option<()> {
    let mut data_maps = BOT_DATA_MAP.get(engine_token).try_borrow_mut().ok()?;
    let bot_data = data_maps.as_mut().get_mut(bot?.pl.index as usize)?; // index and edict should be the same

    bot_data.target_pos = target;

    None
}

#[rrplug::sqfunction(VM = "Server", ExportName = "BotSetSimulationType")]
fn bot_set_sim_type(bot: Option<&mut CPlayer>, sim_type: i32) -> Option<()> {
    let mut data_maps = BOT_DATA_MAP.get(engine_token).try_borrow_mut().ok()?;
    let bot_data = data_maps.as_mut().get_mut(bot?.pl.index as usize)?; // index and edict should be the same

    if sim_type >= 0 {
        bot_data.sim_type = Some(sim_type);
    } else {
        bot_data.sim_type = None;
    }

    None
}

#[rrplug::sqfunction(VM = "Server", ExportName = "BotSpawn")]
fn bot_spawn(bot_name: String) -> Option<i32> {
    spawn_fake_player(
        bot_name.is_empty().then(get_bot_name).unwrap_or(bot_name),
        choose_team(),
        None,
        SERVER_FUNCTIONS.wait(),
        ENGINE_FUNCTIONS.wait(),
        engine_token,
    )
}

#[rrplug::sqfunction(VM = "Server", ExportName = "AddBotName")]
fn add_bot_name(name: String) {
    let mut names = PLUGIN.wait().bots.generic_bot_names.lock();
    if !name.is_empty() {
        names.push(name);
    }
}

#[rrplug::sqfunction(VM = "Server", ExportName = "ClearBotNames")]
fn clear_bot_names() {
    let mut names = PLUGIN.wait().bots.generic_bot_names.lock();
    names.clear();
}

#[rrplug::sqfunction(VM = "Server", ExportName = "RememberNameOverride")]
fn remember_name_override(
    player: Option<&mut CPlayer>,
    name: String,
    clan_tag: String,
) -> Option<()> {
    let engine = ENGINE_FUNCTIONS.get()?;
    let client = unsafe {
        std::slice::from_raw_parts(
            engine.client_array,
            engine.globals.as_ref()?.maxClients as usize,
        )
        .get(player?.pl.index as usize)?
    };

    *PLUGIN
        .wait()
        .bots
        .player_names
        .lock()
        .entry(client.m_UID)
        .or_default() = (name, clan_tag);

    None
}

#[rrplug::sqfunction(VM = "Server", ExportName = "RememberNameOverrideUid")]
fn remember_name_override_uid(uid: String, name: String, clan_tag: String) -> Option<()> {
    *PLUGIN
        .wait()
        .bots
        .player_names
        .lock()
        .entry(std::array::from_fn(|i| {
            uid.as_bytes().get(i).copied().unwrap_or(0) as i8
        }))
        .or_default() = (name, clan_tag);

    None
}
