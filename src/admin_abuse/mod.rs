use rrplug::prelude::*;

use self::{slay::register_slay_command, switch::register_switch_command};

mod slay;
mod switch;

static mut ADMINS: Vec<Box<str>> = Vec::new();

#[derive(Debug)]
pub struct AdminAbuse;

impl Plugin for AdminAbuse {
    fn new(_: &PluginData) -> Self {
        Self {}
    }

    fn on_dll_load(&self, engine_data: Option<&EngineData>, dll_ptr: &DLLPointer) {
        if let WhichDll::Server = dll_ptr.which_dll() {
            // grant_admin should be registered by now or if the mod is not enabled register the convar
            parse_admins(
                ConVarStruct::find_convar_by_name("grant_admin")
                    .unwrap_or_else(register_grant_admin),
            );
        }

        let Some(engine_data) = engine_data else {
            return;
        };

        register_slay_command(engine_data);
        register_switch_command(engine_data);
    }

    fn on_sqvm_created(&self, _sqvm_handle: &CSquirrelVMHandle) {
        let _ = ConVarStruct::find_convar_by_name("grant_admin").map(parse_admins);
    }
}

fn register_grant_admin() -> ConVarStruct {
    log::warn!("grant_admin not detected registering it!");

    let mut convar = ConVarStruct::try_new().expect("something went wrong and convar reg failed!");
    convar
        .register(ConVarRegister::new(
            "grant_admin",
            "1004329002322", // me!
            rrplug::bindings::cvar::convar::FCVAR_GAMEDLL as i32,
            "uids that can use admin commands",
        ))
        .expect("something went wrong and convar reg failed!");

    convar
}

pub fn parse_admins(convar: ConVarStruct) {
    unsafe { &mut ADMINS }.extend(
        convar
            .get_value_string()
            .map_err(|_| {
                log::error!("grant_admins is not utf-8");
                ()
            })
            .unwrap_or_default()
            .split(',')
            .map(|admin| Box::from(admin)),
    );

    log::info!("parsed grant_admins: new admins: {:?}", get_admins());
}

pub fn get_admins() -> &'static [Box<str>] {
    unsafe { &ADMINS }
}
