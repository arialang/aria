// SPDX-License-Identifier: Apache-2.0

use std::{cell::RefCell, rc::Rc};

use haxby_opcodes::function_attribs::FUNC_IS_METHOD;

use crate::{
    arity::Arity,
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
    frame::Frame,
    runtime_value::{
        RuntimeValue,
        function::{BuiltinFunctionImpl, Function},
        object::Object,
        opaque::OpaqueValue,
        structure::Struct,
    },
    symbol::{INTERNED_ATTR_IMPL, INTERNED_ATTR_NEXT},
    vm::RunloopExit,
};

struct EmptyIterator {}
impl Iterator for EmptyIterator {
    type Item = RuntimeValue;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub struct NativeIteratorImpl {
    iter: Rc<RefCell<dyn Iterator<Item = RuntimeValue>>>,
}

impl NativeIteratorImpl {
    pub fn new<T>(iter: T) -> Self
    where
        T: Iterator<Item = RuntimeValue> + 'static,
    {
        Self {
            iter: Rc::new(RefCell::new(iter)),
        }
    }

    pub fn empty() -> Self {
        Self::new(EmptyIterator {})
    }
}

impl Iterator for NativeIteratorImpl {
    type Item = RuntimeValue;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.borrow_mut().next()
    }
}

#[derive(Default)]
struct Next {}
impl BuiltinFunctionImpl for Next {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_this = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let impl_sym = vm
            .globals
            .intern_symbol("__impl")
            .expect("too many symbols interned");
        let iterator_impl = aria_this
            .read(impl_sym)
            .ok_or(VmErrorReason::UnexpectedVmState)?;
        let rust_native_iter = iterator_impl
            .as_opaque_concrete::<RefCell<NativeIteratorImpl>>()
            .ok_or(VmErrorReason::UnexpectedVmState)?;

        if let Some(next) = rust_native_iter.borrow_mut().next() {
            frame.stack.push(vm.globals.create_maybe_some(next)?);
        } else {
            frame.stack.push(vm.globals.create_maybe_none()?);
        }

        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> Arity {
        Arity::required(1)
    }

    fn name(&self) -> &str {
        "next"
    }
}

pub fn create_iterator_struct(
    iter_struct: &Struct,
    imp: NativeIteratorImpl,
    builtins: &mut VmGlobals,
) -> RuntimeValue {
    let obj = RuntimeValue::Object(Object::new(iter_struct));
    let impl_attrib = OpaqueValue::new(RefCell::new(imp));
    obj.write_attribute(
        INTERNED_ATTR_IMPL,
        RuntimeValue::Opaque(impl_attrib),
        builtins,
    )
    .expect("failed to write iterator impl");
    let next = Function::new_builtin::<Next>();
    let bound_next = obj.bind(next);
    obj.write_attribute(INTERNED_ATTR_NEXT, bound_next, builtins)
        .expect("failed to write iterator next");
    obj
}
