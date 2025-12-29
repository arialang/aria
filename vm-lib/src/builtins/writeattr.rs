// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals, error::vm_error::VmErrorReason, frame::Frame,
    runtime_value::function::BuiltinFunctionImpl, vm::RunloopExit,
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
        let the_value = frame.stack.pop();
        let result = the_object.write_attribute(&the_string.raw_value(), the_value);
        match result {
            Ok(_) => {
                frame.stack.push(vm.globals.create_unit_object()?);
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => {
                let er = match e {
                    crate::runtime_value::AttributeError::NoSuchAttribute => {
                        VmErrorReason::NoSuchIdentifier(the_string.raw_value())
                    }
                    crate::runtime_value::AttributeError::InvalidFunctionBinding => {
                        VmErrorReason::InvalidBinding
                    }
                    crate::runtime_value::AttributeError::ValueHasNoAttributes => {
                        VmErrorReason::UnexpectedType
                    }
                };
                Err(er.into())
            }
        }
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
