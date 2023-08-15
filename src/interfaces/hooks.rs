use rrplug::prelude::*;

pub fn hook(dll: &DLLPointer) {
    match dll.which_dll() {
        WhichDll::Engine => {}
        WhichDll::Client => {}
        WhichDll::Server => hook_server(dll),
        WhichDll::Other(_) => {}
    }
}

fn hook_server(_dll: &DLLPointer) {
    // todo
}
