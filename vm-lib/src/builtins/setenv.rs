// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals, error::vm_error::VmErrorReason, frame::Frame,
    runtime_value::function::BuiltinFunctionImpl, vm::RunloopExit,
};

#[derive(Default)]
struct Setenv {}
impl BuiltinFunctionImpl for Setenv {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let var_name = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;
        let var_value = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;
        if var_name.is_empty() || var_value.is_empty() {
            return Err(VmErrorReason::OperationFailed("empty key or value".into()).into());
        }
        if var_name.contains("=") || var_value.contains("=") {
            return Err(VmErrorReason::OperationFailed(
                "key or value contains invalid character '='".into(),
            )
            .into());
        }
        unsafe {
            std::env::set_var(var_name.raw_value(), var_value.raw_value());
        }
        frame.stack.push(vm.globals.create_unit_object()?);
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "setenv"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Setenv>();
}
