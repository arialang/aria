// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl},
    vm::RunloopExit,
};

#[derive(Default)]
struct HasAttr {}
impl BuiltinFunctionImpl for HasAttr {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_value = frame.stack.pop();
        let the_string = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?.raw_value();
        if let Some(symbol) = vm.globals.lookup_symbol(&the_string) {
            let has_attr = the_value.read_attribute(symbol, &vm.globals).is_ok();
            frame.stack.push(RuntimeValue::Boolean(has_attr.into()));
        } else {
            frame.stack.push(RuntimeValue::Boolean(false.into()));
        }

        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "hasattr"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<HasAttr>();
}
