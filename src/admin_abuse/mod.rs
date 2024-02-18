use rrplug::prelude::*;

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
