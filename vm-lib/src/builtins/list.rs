// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::function_attribs::{FUNC_IS_METHOD, METHOD_ATTRIBUTE_TYPE};

use crate::{
    error::vm_error::VmErrorReason,
    frame::Frame,
    runtime_value::{
        RuntimeValue, function::BuiltinFunctionImpl, kind::RuntimeValueType, list::List,
        rust_native_type::RustNativeType,
    },
    vm::RunloopExit,
};

use super::VmGlobals;

#[derive(Default)]
struct ListLen {}
impl BuiltinFunctionImpl for ListLen {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        let len = this.len() as i64;
        frame.stack.push(RuntimeValue::Integer(len.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "len"
    }
}

#[derive(Default)]
struct ListAppend {}
impl BuiltinFunctionImpl for ListAppend {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        let the_value = frame.stack.pop();
        this.append(the_value);
        frame.stack.push(RuntimeValue::List(this));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "append"
    }
}

#[derive(Default)]
struct Drop {}
impl BuiltinFunctionImpl for Drop {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        if this.is_empty() {
            Err(VmErrorReason::IndexOutOfBounds(0).into())
        } else {
            let the_value = this.get_at(this.len() - 1).unwrap();
            this.pop();
            frame.stack.push(the_value);
            Ok(RunloopExit::Ok(()))
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "drop"
    }
}

#[derive(Default)]
struct GetAt {}
impl BuiltinFunctionImpl for GetAt {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        let index = VmGlobals::extract_arg(frame, |x| x.as_integer().cloned())?;
        let index = index.raw_value() as usize;
        match this.get_at(index) {
            Some(v) => {
                frame.stack.push(v);
                Ok(RunloopExit::Ok(()))
            }
            None => Err(VmErrorReason::IndexOutOfBounds(index).into()),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_get_at"
    }
}

#[derive(Default)]
struct SetAt {}
impl BuiltinFunctionImpl for SetAt {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        let index = VmGlobals::extract_arg(frame, |x| x.as_integer().cloned())?;
        let index = index.raw_value() as usize;
        let value = frame.stack.pop();
        match this.set_at(index, value) {
            Ok(_) => {
                frame.stack.push(vm.globals.create_unit_object()?);
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => Err(e.into()),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(3)
    }

    fn name(&self) -> &str {
        "_set_at"
    }
}

#[derive(Default)]
struct NewWithCapacity {}
impl BuiltinFunctionImpl for NewWithCapacity {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let _ = frame.stack.pop(); // ignore List type, we know who we are
        let capacity = VmGlobals::extract_arg(frame, |x| x.as_integer().cloned())?.raw_value();
        let capacity = if capacity < 0 { 0 } else { capacity } as usize;
        let list = List::new_with_capacity(capacity);
        frame.stack.push(RuntimeValue::List(list));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD | METHOD_ATTRIBUTE_TYPE
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "new_with_capacity"
    }
}

pub(super) fn insert_list_builtins(builtins: &mut VmGlobals) {
    let list_builtin =
        RustNativeType::new(crate::runtime_value::rust_native_type::RustNativeValueKind::List);

    list_builtin.insert_builtin::<ListLen>();
    list_builtin.insert_builtin::<ListAppend>();
    list_builtin.insert_builtin::<Drop>();
    list_builtin.insert_builtin::<GetAt>();
    list_builtin.insert_builtin::<SetAt>();
    list_builtin.insert_builtin::<NewWithCapacity>();

    builtins.insert(
        "List",
        RuntimeValue::Type(RuntimeValueType::RustNative(list_builtin)),
    );
}
