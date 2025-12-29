// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals,
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl},
    vm::RunloopExit,
};

#[derive(Default)]
struct CmdlineArgs {}
impl BuiltinFunctionImpl for CmdlineArgs {
    fn eval(
        &self,
        cur_frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let args = vm
            .options
            .vm_args
            .iter()
            .map(|arg| RuntimeValue::String(arg.as_str().into()))
            .collect::<Vec<_>>();
        let args_list = RuntimeValue::List(crate::runtime_value::list::List::from(&args));
        cur_frame.stack.push(args_list);
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::zero()
    }

    fn name(&self) -> &str {
        "cmdline_arguments"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<CmdlineArgs>();
}
