// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
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
        let the_string = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;
        let attr_name = the_string.raw_value();
        let attr_sym = vm
            .globals
            .intern_symbol(&attr_name)
            .map_err(|_| VmErrorReason::UnexpectedVmState)?;
        let has_attr = the_value.read_attribute(attr_sym, &vm.globals).is_ok();
        frame.stack.push(RuntimeValue::Boolean(has_attr.into()));
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
