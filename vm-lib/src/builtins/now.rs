// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl},
    vm::RunloopExit,
};

#[derive(Default)]
struct Now {}
impl BuiltinFunctionImpl for Now {
    fn eval(
        &self,
        cur_frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("before the epoch")
            .as_millis() as i64;
        cur_frame.stack.push(RuntimeValue::Integer(now.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::zero()
    }

    fn name(&self) -> &str {
        "now"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Now>();
}
