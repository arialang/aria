// SPDX-License-Identifier: Apache-2.0
use crate::{
    arity::Arity,
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
    frame::Frame,
    runtime_value::{
        RuntimeValue, function::BuiltinFunctionImpl, kind::RuntimeValueType, object::Object,
    },
    vm::RunloopExit,
};

#[derive(Default)]
struct Alloc {}
impl BuiltinFunctionImpl for Alloc {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        use crate::runtime_value::rust_native_type::RustNativeValueKind as BVK;
        let alloc_type = VmGlobals::extract_arg(frame, |x| x.as_type().cloned())?;

        match alloc_type {
            RuntimeValueType::RustNative(b) => {
                let rv = match b.get_tag() {
                    BVK::Boolean => RuntimeValue::Boolean(false.into()),
                    BVK::Integer => RuntimeValue::Integer(0.into()),
                    BVK::Float => RuntimeValue::Float(0.0.into()),
                    BVK::List => RuntimeValue::List(crate::runtime_value::list::List::from(&[])),
                    BVK::String => RuntimeValue::String("".into()),
                    BVK::Type => return Err(VmErrorReason::UnexpectedType.into()),
                };
                frame.stack.push(rv);
            }
            RuntimeValueType::Struct(s) => {
                let obj = RuntimeValue::Object(Object::new(&s));
                frame.stack.push(obj);
            }
            _ => {
                return Err(VmErrorReason::UnexpectedType.into());
            }
        }

        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> Arity {
        Arity {
            required: 1,
            optional: 0,
        }
    }

    fn name(&self) -> &str {
        "alloc"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Alloc>();
}
