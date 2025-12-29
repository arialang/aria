// SPDX-License-Identifier: Apache-2.0
use crate::{
    builtins::VmGlobals, error::vm_error::VmErrorReason, frame::Frame,
    runtime_value::function::BuiltinFunctionImpl, vm::RunloopExit,
};

#[derive(Default)]
struct ReadAttr {}
impl BuiltinFunctionImpl for ReadAttr {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_value = frame.stack.pop();
        let the_string = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;
        let result = the_value.read_attribute(&the_string.raw_value(), &vm.globals);
        match result {
            Ok(val) => {
                frame.stack.push(val);
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
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "readattr"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<ReadAttr>();
}
