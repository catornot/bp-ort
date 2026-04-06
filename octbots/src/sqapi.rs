use std::thread;

use rrplug::{
    high::{UnsafeHandle, engine_sync, squirrel::SuspendThread},
    mid::squirrel::sqvm_to_context,
    prelude::*,
};
use shared::squtils::{SQOutParam, get_generation, try_get_sqvm_with_generation};

use crate::pathfinding::AreaCost;

pub fn octtree_register_sq_functions() {
    register_sq_functions(octtree_find_path);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "OcttreeFindPath")]
fn octtree_find_path(
    start: Vector3,
    end: Vector3,
    out_param: SQOutParam<Vec<Vector3>>,
) -> Result<SuspendThread<()>, String> {
    let Some(recv) = crate::PLUGIN.wait().job_market.find_path(
        start,
        crate::async_pathfinding::GoalFloat::ClosestToPoint(end),
        AreaCost::default(),
    ) else {
        return Err("couldn't start pathfinding".to_string());
    };

    let (suspend, Some(resume)) = SuspendThread::new_both(sqvm) else {
        return Err("this function was called without SpinOff()")?;
    };

    let context = unsafe { sqvm_to_context(sqvm) };
    let generation = get_generation(context);

    // should maybe return more info about the error but Result doesn't have PushToSquirrelVm implemented ...
    let resume = unsafe { UnsafeHandle::new(resume) };
    let out_param = unsafe { UnsafeHandle::new(out_param) };
    thread::spawn(move || {
        let path = match recv.recv() {
            Ok(Some(path)) => path
                .into_iter()
                .map(|nav_point| nav_point.as_vec())
                .collect(),
            Err(_) | Ok(None) => Vec::new(),
        };

        _ = engine_sync::async_execute(AsyncEngineMessage::run_func(move |token| {
            let Some(sqvm) = try_get_sqvm_with_generation(generation, context, token) else {
                log::warn!("called load file async on wrong generation");
                return;
            };

            out_param.take().set_out_var(path, sqvm, sq_functions);
            resume.take().resume(());
        }))
    });

    Ok(suspend)
}
