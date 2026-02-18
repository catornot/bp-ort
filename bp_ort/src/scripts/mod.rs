use rrplug::{
    bindings::cvar::{command::CCommand, convar::FCVAR_GAMEDLL},
    prelude::*,
};

#[derive(Debug)]
pub struct Scripts;

impl Plugin for Scripts {
    const PLUGIN_INFO: PluginInfo =
        PluginInfo::new(c"scripts", c"scripts", c"scripts", PluginContext::DEDICATED);
    fn new(_: bool) -> Self {
        Self {}
    }

    fn on_dll_load(
        &self,
        engine_data: Option<&EngineData>,
        _dll_ptr: &DLLPointer,
        engine_token: EngineToken,
    ) {
        let Some(engine_data) = engine_data else {
            return;
        };

        engine_data
            .register_concommand(
                "ss",
                script_run_server,
                "todo",
                FCVAR_GAMEDLL as i32,
                engine_token,
            )
            .expect("could not create ss");
    }
}

unsafe extern "C" fn script_run_server(command: *const CCommand) {
    unsafe {
        let command = command
            .as_ref()
            .expect("CCommand should be valid in concommands");
        let script = String::from_utf8_lossy(
            std::mem::transmute::<_, [u8; 512]>(command.m_pArgSBuffer).as_slice(),
        )
        .to_string();

        log::info!("todo wow: {script}")
    };
}

// #[rrplug::sqfunction(VM = "Server | Client | Ui", ExportName = "printstruct")]
// fn print_struct(sqstruct: SQObject) -> bool {
//     if sqstruct._Type != SQObjectType::OT_STRUCT {
//         // || sqstruct._Type != SQObjectType::OT_INSTANCE
//         return false;
//     }
//     let Some(sqstruct) = (unsafe { sqstruct._VAL.asStructInstance.as_mut() }) else {
//         return false;
//     };

//     log::info!("struct {{ {} }}", unsafe {
//         (0..sqstruct.size as usize).map(|i| sqstruct.data.as_ptr().add(i).as_ref())
//     });

//     true
// }
