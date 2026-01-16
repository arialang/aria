// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals, frame::Frame, runtime_value::function::BuiltinFunctionImpl,
    vm::RunloopExit,
};

#[derive(Default)]
struct Exit {}
impl BuiltinFunctionImpl for Exit {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let code = VmGlobals::extract_arg(frame, |x| x.as_integer().cloned())?;
        std::process::exit(*code.raw_value() as i32);
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "exit"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Exit>();
}
