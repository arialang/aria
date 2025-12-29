// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl},
    vm::RunloopExit,
};

#[derive(Default)]
struct Getenv {}
impl BuiltinFunctionImpl for Getenv {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let var_name = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;
        match std::env::var(var_name.raw_value()).map(|s| RuntimeValue::String(s.into())) {
            Ok(s) => match vm.globals.create_maybe_some(s) {
                Ok(s) => {
                    frame.stack.push(s);
                    Ok(RunloopExit::Ok(()))
                }
                Err(e) => Err(e.into()),
            },
            Err(_) => match vm.globals.create_maybe_none() {
                Ok(s) => {
                    frame.stack.push(s);
                    Ok(RunloopExit::Ok(()))
                }
                Err(e) => Err(e.into()),
            },
        }
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "getenv"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Getenv>();
}
