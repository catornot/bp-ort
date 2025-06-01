use parking_lot::{RwLock, RwLockReadGuard};
use rrplug::{
    bindings::class_types::{client::SignonState, cplayer::CPlayer},
    prelude::*,
};

use crate::{
    bindings::{EngineFunctions, ServerFunctions, ENGINE_FUNCTIONS},
    utils::{from_c_string, get_c_char_array_lossy, iterate_c_array_sized, send_client_print},
};

use self::{
    grant_admin::register_grant_admin_command, health::register_health_command,
    slay::register_slay_command, switch::register_switch_command,
    teleport::register_teleport_command,
};

mod grant_admin;
mod health;
mod slay;
mod switch;
mod teleport;

static ADMINS: RwLock<Vec<Box<str>>> = RwLock::new(Vec::new());

#[derive(Debug)]
pub struct AdminAbuse;

impl Plugin for AdminAbuse {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        c"AdminAbuse",
        c"ADMINABUSE",
        c"AdminAbuse",
        PluginContext::all(),
    );

    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_dll_load(
        &self,
        engine_data: Option<&EngineData>,
        dll_ptr: &DLLPointer,
        token: EngineToken,
    ) {
        if let WhichDll::Server = dll_ptr.which_dll() {
            // grant_admin should be registered by now or if the mod is not enabled register the convar
            parse_admins(
                ConVarStruct::find_convar_by_name("grant_admin", token)
                    .unwrap_or_else(|_| register_grant_admin(token)),
            );
        }

        let Some(engine_data) = engine_data else {
            return;
        };

        register_slay_command(engine_data, token);
        register_switch_command(engine_data, token);
        register_teleport_command(engine_data, token);
        register_health_command(engine_data, token);
        register_grant_admin_command(engine_data, token);
    }

    fn on_sqvm_created(&self, _sqvm_handle: &CSquirrelVMHandle, token: EngineToken) {
        let _ = ConVarStruct::find_convar_by_name("grant_admin", token).map(parse_admins);
    }
}

fn register_grant_admin(token: EngineToken) -> ConVarStruct {
    log::warn!("grant_admin not detected registering it!");

    ConVarStruct::try_new(
        &ConVarRegister::new(
            "grant_admin",
            "1004329002322", // me!
            rrplug::bindings::cvar::convar::FCVAR_GAMEDLL as i32,
            "uids that can use admin commands",
        ),
        token,
    )
    .expect("something went wrong and convar reg failed!")
}

fn parse_admins(convar: ConVarStruct) {
    {
        let mut admins = ADMINS.write();
        admins.clear();
        admins.extend(
            convar
                .get_value_str()
                .map_err(|_| {
                    log::error!("grant_admins is not utf-8");
                })
                .unwrap_or_default()
                .split(',')
                .map(Box::from),
        );
    }
    log::info!("parsed grant_admins: new admins: {:?}", get_admins());
}

pub fn get_admins<'a>() -> RwLockReadGuard<'a, Vec<Box<str>>> {
    ADMINS.read()
}

pub fn filter_target(filter: Option<&str>, player: &CPlayer, name: &str) -> bool {
    match filter {
        Some("all") => true,
        Some("imc") => player.m_iTeamNum == 2,
        Some("militia") => player.m_iTeamNum == 3,
        Some(fname) => name.starts_with(fname),
        None => false,
    }
}

// clippy just doesn't get it
#[allow(clippy::mut_from_ref)]
pub fn admin_check<'a, 'b>(
    command: &'a CCommandResult,
    engine_funcs: &'b EngineFunctions,
    server_funcs: &'b ServerFunctions,
) -> (bool, Option<&'a mut CPlayer>) {
    let caller_player = unsafe { (server_funcs.util_get_command_client)().as_mut() };

    let has_admin = caller_player
        .as_ref()
        .and_then(|caller_player| unsafe {
            engine_funcs
                .client_array
                .add(caller_player.pl.index.saturating_sub(1) as usize)
                .as_ref()
                .map(|c| from_c_string::<String>(c.m_UID.as_ptr()))
        })
        .map(|uid| get_admins().iter().any(|admin| admin.as_ref() == uid))
        .unwrap_or(true);

    if !has_admin {
        log::warn!(
            "Client needs to have admin to run {}",
            command.get_command()
        );
        if let Some(admin) = caller_player.as_ref() {
            _ = send_client_print(
                admin,
                &format!(
                    "You need to be an admin on this server to run {}",
                    command.get_command()
                ),
            )
        }
    }

    (has_admin, caller_player)
}

#[rrplug::concommand]
pub fn forward_to_server(command: CCommandResult) {
    unsafe {
        let engine = ENGINE_FUNCTIONS.wait();
        let cmd = format!(
            "{}_server {}\0",
            command.get_command(),
            command
                .get_args()
                .iter()
                .cloned()
                .map(|s| s + " ")
                .collect::<String>()
        );
        let cmd_ptr = cmd.as_ptr() as *const libc::c_char;

        (engine.cengine_client_server_cmd)(std::ptr::null_mut(), cmd_ptr, true);
    }
}

pub fn execute_for_matches(
    filter: Option<&str>,
    execution: impl Fn(&mut CPlayer),
    should_live: bool,
    server_funcs: &ServerFunctions,
    engine_funcs: &EngineFunctions,
) {
    unsafe { iterate_c_array_sized::<_, 32>(engine_funcs.client_array.into()) }
        .enumerate()
        .filter(|(_, client)| client.m_nSignonState == SignonState::FULL)
        .filter_map(|(e, client)| unsafe {
            Some((
                (server_funcs.get_player_by_index)(e as i32 + 1).as_mut()?,
                get_c_char_array_lossy(&client.m_szServerName),
            ))
        })
        .filter(|(player, _)| unsafe { !should_live || (server_funcs.is_alive)(*player) != 0 })
        .filter(|(player, name)| filter_target(filter, player, name))
        .for_each(|(player, _)| execution(player));
}
