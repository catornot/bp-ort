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
