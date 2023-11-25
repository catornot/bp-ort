use rrplug::{
    bindings::cvar::command::{COMMAND_COMPLETION_ITEM_LENGTH, COMMAND_COMPLETION_MAXITEMS},
    mid::{concommands::find_concommand, engine::get_engine_data},
    prelude::*,
};
use std::ffi::c_char;
use utils::from_c_string;

mod bindings;
mod bots;
mod disguise;
mod interfaces;
mod utils;

use crate::{
    bindings::{
        ClientFunctions, EngineFunctions, MatSysFunctions, ServerFunctions, CLIENT_FUNCTIONS,
        ENGINE_FUNCTIONS, MATSYS_FUNCTIONS, SERVER_FUNCTIONS,
    },
    bots::Bots,
    disguise::Disguise,
    interfaces::Interfaces,
    screen_detour::hook_materialsystem,
    utils::set_c_char_array,
};

mod screen_detour;

#[derive(Debug)]
pub struct HooksPlugin {
    pub bots: Bots,
    pub disguise: Disguise,
    pub interfaces: Interfaces,
}

impl Plugin for HooksPlugin {
    fn new(plugin_data: &PluginData) -> Self {
        Self {
            bots: Bots::new(plugin_data),
            disguise: Disguise::new(plugin_data),
            interfaces: Interfaces::new(plugin_data),
        }
    }

    fn on_dll_load(&self, engine: Option<&EngineData>, dll_ptr: &DLLPointer) {
        self.bots.on_dll_load(engine, dll_ptr);
        self.disguise.on_dll_load(engine, dll_ptr);
        self.interfaces.on_dll_load(engine, dll_ptr);

        unsafe {
            EngineFunctions::try_init(dll_ptr, &ENGINE_FUNCTIONS);
            ClientFunctions::try_init(dll_ptr, &CLIENT_FUNCTIONS);
            ServerFunctions::try_init(dll_ptr, &SERVER_FUNCTIONS);
            MatSysFunctions::try_init(dll_ptr, &MATSYS_FUNCTIONS);
        }

        match dll_ptr.which_dll() {
            WhichDll::Other(other) if *other == "materialsystem_dx11.dll" => {
                hook_materialsystem(dll_ptr.get_dll_ptr())
            }
            // PluginLoadDLL::Server => unsafe {
            //     let base = SERVER_FUNCTIONS.wait().base as usize;
            //     // patch(
            //     //     base + 0x5a8241,
            //     //     &[
            //     //         0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
            //     //         0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
            //     //         0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
            //     //     ],
            //     // ); // removes the Client \'%s\' dropped %i packets to us spam
            //     // patch(
            //     //     base + 0x5a825d,
            //     //     &[
            //     //         0x90, 0x90, 0x90, 0x90, 0x90,
            //     //     ],
            //     // ); // same thing but less nops
            // },
            WhichDll::Server => unsafe {
                let concommand = get_engine_data()
                    .unwrap()
                    .register_concommand("test_completion", test_completion, "", 0)
                    .expect("couldn't register concommand test_completion");

                // let command = rrplug::mid::northstar::CREATE_OBJECT_FUNC.wait().unwrap()(
                //     rrplug::bindings::plugin_abi::ObjectType::CONCOMMANDS,
                // ) as *mut ConCommand;
                // let concommand = (SERVER_FUNCTIONS.wait().register_con_command)(
                //     command,
                //     to_c_string!(const "test_completion\0").as_ptr(),
                //     Some(test_completion),
                //     std::ptr::null(),
                //     0,
                //     test_completion_completion,
                // );
                log::info!(
                    "test_completion m_nCallbackFlags {}",
                    (*concommand).m_nCallbackFlags
                );

                (*concommand).m_pCompletionCallback = Some(test_completion_completion);

                // (*concommand).m_nCallbackFlags = true as i32;
                (*concommand).m_nCallbackFlags =
                    true as i32 | (*concommand).m_nCallbackFlags & 0xfa | 2;

                // super bad
                // (*concommand).m_nCallbackFlags = (*concommand).m_nCallbackFlags | true as i32 | 4;
                let flags = find_concommand("give").unwrap().m_nCallbackFlags;
                log::info!("give flags {:#10x}, {:#10b}", flags, flags);
            },
            _ => {}
        }
    }

    fn on_sqvm_created(&self, _sqvm_handle: &CSquirrelVMHandle) {
        // self.bots.on_sqvm_created(sqvm_handle)
    }

    fn on_sqvm_destroyed(&self, context: ScriptVmType) {
        self.bots.on_sqvm_destroyed(context)
    }

    fn runframe(&self) {
        self.interfaces.runframe()
    }
}

#[rrplug::concommand]
pub fn test_completion(command: CCommandResult) -> Option<()> {
    let arg = command.get_args().get(0)?;
    log::info!("arg {arg}");

    None
}

pub(crate) unsafe extern "C" fn test_completion_completion(
    partial: *const c_char,
    commands: *mut [c_char; COMMAND_COMPLETION_ITEM_LENGTH as usize],
) -> i32 {
    let cmd = from_c_string::<String>(partial);
    let cmd = cmd.split_once(' ').map(|(cmd, _)| cmd).unwrap_or(&cmd);

    let commands = std::slice::from_raw_parts_mut(commands, COMMAND_COMPLETION_MAXITEMS as usize);

    set_c_char_array(
        &mut commands[0],
        &format!("{cmd} hksdgkshgskjghskjghsjkghksg\0"),
    );
    set_c_char_array(&mut commands[1], &format!("{cmd} test3\0"));
    set_c_char_array(&mut commands[2], &format!("{cmd} test2\0"));
    set_c_char_array(&mut commands[3], &format!("{cmd} test4\0"));
    set_c_char_array(
        &mut commands[4],
        &format!("{cmd} 239852789472895798272592\0"),
    );
    set_c_char_array(&mut commands[5], &format!("{cmd} yo completion?\0"));

    5
}

entry!(HooksPlugin);
