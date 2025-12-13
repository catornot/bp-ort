use rrplug::{high::squirrel::SuspendThread, prelude::*};

use crate::pathfinding::AreaCost;

pub fn octtree_register_sq_functions() {
    register_sq_functions(octtree_find_path);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "OcttreeFindPath")]
fn octtree_find_path(start: Vector3, end: Vector3) -> Result<SuspendThread<Vec<Vector3>>, String> {
    let Some(recv) = crate::PLUGIN
        .wait()
        .job_market
        .find_path(start, end, AreaCost::default())
    else {
        return Err("couldn't start pathfinding".to_string());
    };

    // should maybe return more info about the error but Result doesn't have PushToSquirrelVm implemented ...
    Ok(SuspendThread::new_with_thread(sqvm, move || {
        match recv.recv() {
            Ok(Some(path)) => path
                .into_iter()
                .map(|nav_point| nav_point.as_vec())
                .collect(),
            Err(_) | Ok(None) => Vec::new(),
        }
    }))
}
