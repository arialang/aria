// SPDX-License-Identifier: Apache-2.0
use std::io::Write;

use crate::{
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl},
    vm::RunloopExit,
};

#[derive(Default)]
struct Readln {}
impl BuiltinFunctionImpl for Readln {
    fn eval(
        &self,
        cur_frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_value = cur_frame.stack.pop();
        let the_value = the_value.prettyprint(cur_frame, vm);
        let _ = std::io::stdout().write(the_value.as_bytes());
        let _ = std::io::stdout().flush();

        let mut input = String::new();
        let result = std::io::stdin().read_line(&mut input);
        match result {
            Ok(_) => {
                let input = input.trim();
                cur_frame.stack.push(RuntimeValue::String(input.into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => Err(VmErrorReason::OperationFailed(e.to_string()).into()),
        }
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "readln"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Readln>();
}
