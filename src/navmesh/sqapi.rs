use std::{cell::RefCell, ptr::NonNull};

use rrplug::{
    bindings::squirrelclasstypes::SQRESULT,
    high::{
        squirrel::{UserData, UserDataRef},
        squirrel_traits::PushToSquirrelVm,
    },
    prelude::*,
};

use crate::{
    bindings::SERVER_FUNCTIONS,
    navmesh::{
        navigation::{Navigation, NavigationError},
        Hull, RECAST_DETOUR,
    },
};

pub fn navigation_register_sq_functions() {
    register_sq_functions(navigation_new);
    register_sq_functions(navigation_find_path);
    register_sq_functions(navigation_get_all_points);
    register_sq_functions(navigation_next_point);
    register_sq_functions(test_thread);
    register_sq_functions(unfreeze_thread);
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationCreate")]
fn navigation_new(hull: Hull) -> Option<UserData<Navigation>> {
    Some(UserData::new(Navigation::new(hull)?))
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationFindPath")]
fn navigation_find_path(
    mut nav: UserDataRef<Navigation>,
    start: Vector3,
    end: Vector3,
) -> Result<(), NavigationError> {
    nav.new_path(start, end, RECAST_DETOUR.wait())
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationGetAllPoints")]
fn navigation_get_all_points(nav: UserDataRef<Navigation>) -> Vec<Vector3> {
    nav.path_points.clone()
}

#[rrplug::sqfunction(VM = "SERVER", ExportName = "NavigationNextPoint")]
fn navigation_next_point(mut nav: UserDataRef<Navigation>) -> Option<Vector3> {
    nav.next_point()
}

#[allow(clippy::type_complexity)]
static SLEEPING_THREAD: EngineGlobal<RefCell<Option<(NonNull<HSquirrelVM>, i32)>>> =
    EngineGlobal::new(RefCell::new(None));

#[rrplug::sqfunction(VM = "SERVER", ReturnOverwrite = "i32")]
fn test_thread(num: i32) -> Result<(), String> {
    let sv_funcs = SERVER_FUNCTIONS.wait();

    if SLEEPING_THREAD.get(engine_token).borrow().is_some() {
        return Err("this api cannot handle two sleeping threads".to_string());
    }

    unsafe {
        // let offset_into_data = *(&(*(sqvm.as_ref().sharedState)).enableDebugInfo as *const bool
        //     as *mut usize)
        //     .offset(0x3c)
        //     * 0x3a8;
        // let data_pointer = *sv_funcs
        //     .some_global_for_threads
        //     .add(offset_into_data)
        //     .cast::<*mut *mut ()>();
        // let currentThread_maybe = (*data_pointer) as *mut *mut HSquirrelVM;
        // // if !currentThread_maybe.is_null()
        // //     && sqvm.as_ptr() as usize == (*currentThread_maybe) as usize
        // {
        //     let pHVar1 = (sv_funcs.fun_180042560)(data_pointer, 100.);
        //     (*currentThread_maybe.add(0xb))
        //         .cast::<*const i32>()
        //         .write(&(*pHVar1).uiRef);
        //     (*currentThread_maybe.add(10))
        //         .cast::<*mut ()>()
        //         .write((*pHVar1).pointer_58.cast::<()>());
        //     (*pHVar1)
        //         .pointer_58
        //         .cast::<*const *mut HSquirrelVM>()
        //         .write(currentThread_maybe);
        //     (**currentThread_maybe.add(10)).pointer_58 = currentThread_maybe.cast();
        //     (*currentThread_maybe.add(2)) = pHVar1.cast_mut();
        //     (sv_funcs.sq_suspendthread)(sqvm.as_ptr(), *data_pointer.cast(), 2, sqvm.as_ptr());
        //     // return;
        //     log::info!("hmm");
        // }
        // (sv_funcs.somehow_suspend_thread)(sqvm.as_ptr());
        assert_ne!(
            (sv_funcs.sq_suspendthread)(sqvm.as_ptr(), &sqvm.as_ptr().cast(), 5, sqvm.as_ptr()),
            SQRESULT::SQRESULT_ERROR
        );
    };

    SLEEPING_THREAD
        .get(engine_token)
        .borrow_mut()
        .replace((sqvm, num + 5));

    Ok(())
}

#[rrplug::sqfunction(VM = "SERVER")]
fn unfreeze_thread() -> Option<()> {
    let sv_funcs = SERVER_FUNCTIONS.wait();

    let (thread_sqvm, num) = SLEEPING_THREAD.get(engine_token).borrow_mut().take()?;

    num.push_to_sqvm(thread_sqvm, sq_functions);

    unsafe {
        assert_ne!(
            (sv_funcs.sq_threadwakeup)(thread_sqvm.as_ptr(), 5, std::ptr::null(), sqvm.as_ptr()),
            SQRESULT::SQRESULT_ERROR
        );
    };

    Some(())
}
