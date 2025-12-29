// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals, frame::Frame, runtime_value::function::BuiltinFunctionImpl,
    vm::RunloopExit,
};

#[derive(Default)]
struct Print {}
impl BuiltinFunctionImpl for Print {
    fn eval(
        &self,
        cur_frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_value = cur_frame.stack.pop();
        let fmt = the_value.prettyprint(cur_frame, vm);
        let mut console = vm.console().borrow_mut();
        assert!(console.print(&fmt).is_ok());

        cur_frame.stack.push(vm.globals.create_unit_object()?);
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "print"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Print>();
}
