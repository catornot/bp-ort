use rrplug::{bindings::class_types::cplayer::CPlayer, prelude::*};

use crate::{
    bindings::{EngineFunctions, ServerFunctions},
    utils::from_c_string,
};

use self::{slay::register_slay_command, switch::register_switch_command};

mod slay;
mod switch;

static mut ADMINS: Vec<Box<str>> = Vec::new();

#[derive(Debug)]
pub struct AdminAbuse;

impl Plugin for AdminAbuse {
    const PLUGIN_INFO: PluginInfo = PluginInfo::new(
        "AdminAbuse\0",
        "ADMINABUSE\0",
        "AdminAbuse\0",
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

pub fn parse_admins(convar: ConVarStruct) {
    unsafe { &mut ADMINS }.extend(
        convar
            .get_value_str()
            .map_err(|_| {
                log::error!("grant_admins is not utf-8");
            })
            .unwrap_or_default()
            .split(',')
            .map(Box::from),
    );

    log::info!("parsed grant_admins: new admins: {:?}", get_admins());
}

pub fn get_admins() -> &'static [Box<str>] {
    unsafe { &ADMINS }
}

pub fn filter_target(filter: Option<&str>, player: &CPlayer, name: &str) -> bool {
    match filter {
        Some("all") => true,
        Some("imc") => unsafe { *player.team.get_inner() == 2 },
        Some("militia") => unsafe { *player.team.get_inner() == 3 },
        Some(fname) => name.starts_with(fname),
        None => false,
    }
}

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
                .add(caller_player.player_index.copy_inner().saturating_sub(1) as usize)
                .as_ref()
                .map(|c| from_c_string::<String>(c.uid.as_ptr()))
        })
        .map(|uid| get_admins().iter().any(|admin| admin.as_ref() == uid))
        .unwrap_or(true);

    if !has_admin {
        log::warn!(
            "Client needs to have admin to run {}",
            command.get_command()
        );
    }

    (has_admin, caller_player)
}
