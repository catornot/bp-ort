use rrplug::{bindings::convar::FCVAR_GAMEDLL, wrappers::engine::EngineData};

pub fn register_debug_concommands(engine: &EngineData) {
    engine
        .register_concommand(
            "peak_connected_clients",
            peak_connected_clients,
            "peaks the info about connected clients (dumps a lot of info)",
            FCVAR_GAMEDLL as i32,
        )
        .expect("couldn't register concommand peak_connected_clients");

    engine
        .register_concommand(
            "bots_peak",
            bots_peak,
            "",
            FCVAR_GAMEDLL as i32,
        )
        .expect("couldn't register concommand bots_peak");

    engine
        .register_concommand(
            "bot_find",
            bot_find,
            "",
            FCVAR_GAMEDLL as i32,
        )
        .expect("couldn't register concommand bot_find");
}

#[rrplug::concommand]
pub fn peak_connected_clients(command: CCommandResult) {
    crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .client_array
        .peak_array();
}

#[rrplug::concommand]
pub fn bot_find(command: CCommandResult) {
    let name = match command.args.get(0) {
        Some(n) => n,
        None => return,
    };

    let found_client = crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .client_array
        .find(|c| &c.get_name() == name);

    if let Some(c) = found_client {
        c.peak()
    }
}

#[rrplug::concommand]
pub fn bots_peak(command: CCommandResult) {
    for client in (&mut crate::PLUGIN
        .wait()
        .source_engine_data
        .lock()
        .unwrap()
        .client_array)
        .filter(|c| c.is_fake_player())
    {
        client.peak()
    }
}
