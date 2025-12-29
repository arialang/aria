// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl, kind::RuntimeValueType},
    vm::RunloopExit,
};

#[derive(Default)]
struct Typeof {}
impl BuiltinFunctionImpl for Typeof {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_value = frame.stack.pop();
        let the_type = RuntimeValueType::get_type(&the_value, &vm.globals);
        frame.stack.push(RuntimeValue::Type(the_type));
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "typeof"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Typeof>();
}
