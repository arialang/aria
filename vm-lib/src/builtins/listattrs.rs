// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl, list::List},
    vm::RunloopExit,
};

#[derive(Default)]
struct ListAttrs {}
impl BuiltinFunctionImpl for ListAttrs {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_value = frame.stack.pop();
        let attrs = the_value.list_attributes(&vm.globals);
        frame.stack.push(RuntimeValue::List(List::from(
            &attrs
                .iter()
                .map(|x| RuntimeValue::String(x.clone().into()))
                .collect::<Vec<_>>(),
        )));
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "listattrs"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<ListAttrs>();
}
