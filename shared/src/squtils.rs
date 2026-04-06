use rrplug::{
    bindings::squirreldatatypes::{SQArray, SQObject, SQObjectType, SQObjectValue},
    high::{
        squirrel::SQHandle,
        squirrel_traits::{GetFromSquirrelVm, PushToSquirrelVm, SQVMName},
    },
    mid::squirrel::{
        SQVM_CLIENT, SQVM_CLIENT_GENERATION, SQVM_SERVER, SQVM_SERVER_GENERATION, SQVM_UI,
        SQVM_UI_GENERATION,
    },
    prelude::*,
};
use std::{marker::PhantomData, mem::MaybeUninit, ptr::NonNull, sync::atomic::Ordering};

pub struct SQOutParam<T>(Option<SQHandle<'static, SQArray>>, PhantomData<T>);

impl<T: SQVMName> SQVMName for SQOutParam<T> {
    fn get_sqvm_name() -> String {
        format!("array< {} >", T::get_sqvm_name())
    }
}

impl<T> GetFromSquirrelVm for SQOutParam<T> {
    fn get_from_sqvm(
        sqvm: std::ptr::NonNull<HSquirrelVM>,
        sqfunctions: &'static SquirrelFunctions,
        stack_pos: i32,
    ) -> Self {
        let object = SQObject::get_from_sqvm(sqvm, sqfunctions, stack_pos);
        match object._Type {
            SQObjectType::OT_ARRAY => SQOutParam(
                Some(unsafe { SQHandle::new_unchecked(object) }),
                PhantomData,
            ),
            SQObjectType::OT_NULL => SQOutParam(None, PhantomData),
            _ => panic!("how did an array type get in here"),
        }
    }
}

impl<T: PushToSquirrelVm> SQOutParam<T> {
    pub fn set_out_var(
        self,
        out: T,
        mut sqvm: std::ptr::NonNull<HSquirrelVM>,
        sqfunctions: &'static SquirrelFunctions,
    ) -> bool {
        if let SQOutParam(Some(mut array), _) = self {
            let array = array.get_mut();
            if array._allocated < 1 {
                let object = SQObject {
                    _Type: SQObjectType::OT_NULL,
                    structNumber: 0,
                    _VAL: SQObjectValue {
                        asString: std::ptr::null_mut(),
                    },
                };
                unsafe {
                    (sqfunctions.sq_object_vector_resize)(array, 1, &object);
                }
            }

            out.push_to_sqvm(sqvm, sqfunctions);

            unsafe {
                let sqvm = sqvm.as_mut();
                let top = sqvm
                    ._stack
                    .add(sqvm._top as usize - 1)
                    .as_mut()
                    .unwrap_unchecked();
                std::mem::swap(top, array._values.as_mut().unwrap_unchecked());
            };
            array._usedSlots = array._usedSlots.max(1);

            // pop the null object
            unsafe { (sqfunctions.sq_poptop)(sqvm.as_ptr()) };

            true
        } else {
            false
        }
    }
}

pub fn get_generation(context: ScriptContext) -> u32 {
    match context {
        ScriptContext::SERVER => SQVM_SERVER_GENERATION.load(Ordering::Acquire),
        ScriptContext::CLIENT => SQVM_CLIENT_GENERATION.load(Ordering::Acquire),
        ScriptContext::UI => SQVM_UI_GENERATION.load(Ordering::Acquire),
    }
}

pub fn try_get_sqvm_with_generation(
    generation: u32,
    context: ScriptContext,
    token: EngineToken,
) -> Option<NonNull<HSquirrelVM>> {
    match context {
        ScriptContext::SERVER
            if SQVM_SERVER_GENERATION.load(Ordering::Acquire) == generation
                && let Some(sqvm) = SQVM_SERVER.get(token).borrow().as_ref().copied() =>
        {
            Some(sqvm)
        }
        ScriptContext::CLIENT
            if SQVM_CLIENT_GENERATION.load(Ordering::Acquire) == generation
                && let Some(sqvm) = SQVM_CLIENT.get(token).borrow().as_ref().copied() =>
        {
            Some(sqvm)
        }
        ScriptContext::UI
            if SQVM_UI_GENERATION.load(Ordering::Acquire) == generation
                && let Some(sqvm) = SQVM_UI.get(token).borrow().as_ref().copied() =>
        {
            Some(sqvm)
        }
        _ => None,
    }
}
