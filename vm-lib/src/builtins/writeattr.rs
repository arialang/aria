// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals, frame::Frame, runtime_value::function::BuiltinFunctionImpl,
    vm::RunloopExit,
};

#[derive(Default)]
struct WriteAttr {}
impl BuiltinFunctionImpl for WriteAttr {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_object = frame.stack.pop();
        let the_string = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;
        let the_symbol = vm.globals.intern_symbol(the_string.raw_value())?;
        let the_value = frame.stack.pop();
        the_object
            .write_attribute(the_symbol, the_value, &mut vm.globals)
            .map_err(|e| e.to_vm_error_reason(the_string.raw_value()))?;
        frame.stack.push(vm.globals.create_unit_object()?);
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(3)
    }

    fn name(&self) -> &str {
        "writeattr"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<WriteAttr>();
}
