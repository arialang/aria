// SPDX-License-Identifier: Apache-2.0
use std::cell::OnceCell;

use crate::{
    builtins::VmGlobals,
    error::vm_error::{VmError, VmErrorReason},
    frame::Frame,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl, object::Object},
    vm::RunloopExit,
};

struct Cache {
    arity_struct: crate::runtime_value::structure::Struct,
    upper_bound_enum: crate::runtime_value::enumeration::Enum,
    bounded_idx: usize,
    vararg_idx: usize,
}

#[derive(Default)]
struct Arity {
    cache: OnceCell<Cache>,
}
impl Arity {
    fn fill_in_cache(&self, vm: &mut crate::vm::VirtualMachine) -> Result<&Cache, VmError> {
        if let Some(cache) = self.cache.get() {
            Ok(cache)
        } else {
            let arity_mod = vm.find_imported_module("aria.core.arity").ok_or_else(|| {
                VmErrorReason::ImportNotAvailable(
                    "aria.core.arity".to_owned(),
                    "module not found".to_owned(),
                )
            })?;
            let arity_struct = arity_mod.load_named_value("Arity").ok_or_else(|| {
                VmErrorReason::NoSuchIdentifier("aria.core.arity.Arity".to_owned())
            })?;
            let arity_struct = arity_struct
                .as_struct()
                .ok_or(VmErrorReason::UnexpectedType)?;
            let upper_bound_enum =
                arity_struct.load_named_value("UpperBound").ok_or_else(|| {
                    VmErrorReason::NoSuchIdentifier("aria.core.arity.Arity.UpperBound".to_owned())
                })?;
            let upper_bound_enum = upper_bound_enum
                .as_enum()
                .ok_or(VmErrorReason::UnexpectedType)?;

            let vararg_idx = upper_bound_enum
                .get_idx_of_case("Varargs")
                .ok_or_else(|| VmErrorReason::NoSuchCase("Varargs".to_owned()))?;

            let bounded_idx = upper_bound_enum
                .get_idx_of_case("Bounded")
                .ok_or_else(|| VmErrorReason::NoSuchCase("Bounded".to_owned()))?;

            let cache = Cache {
                arity_struct: arity_struct.clone(),
                upper_bound_enum: upper_bound_enum.clone(),
                bounded_idx,
                vararg_idx,
            };

            let _ = self.cache.set(cache);
            Ok(self.cache.get().unwrap())
        }
    }
}

fn get_to_function_for_callable(
    val: &RuntimeValue,
    vm: &mut crate::vm::VirtualMachine,
) -> Option<(crate::runtime_value::function::Function, bool)> {
    if let Some(f) = val.as_function() {
        Some((f.clone(), false))
    } else if let Some(bf) = val.as_bound_function() {
        Some((bf.func().clone(), true))
    } else if let Ok(call) = val.read_attribute("_op_impl_call", &vm.globals) {
        get_to_function_for_callable(&call, vm)
    } else {
        None
    }
}

impl BuiltinFunctionImpl for Arity {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let (f_this, has_receiver) =
            VmGlobals::extract_arg(frame, |val| get_to_function_for_callable(&val, vm))?;
        let arity_cache = self.fill_in_cache(vm)?;

        let f_arity = f_this.arity();
        let is_vararg = f_this.attribute().is_vararg();

        let argc_offset = if has_receiver { 1 } else { 0 };

        let upper_bound_value = if is_vararg {
            arity_cache
                .upper_bound_enum
                .make_value(arity_cache.vararg_idx, None)
        } else {
            arity_cache.upper_bound_enum.make_value(
                arity_cache.bounded_idx,
                Some(RuntimeValue::Integer(
                    ((f_arity.optional + f_arity.required - argc_offset) as i64).into(),
                )),
            )
        }
        .ok_or(VmErrorReason::UnexpectedType)?;

        let upper_bound_value = RuntimeValue::EnumValue(upper_bound_value);

        let lower_bound_value =
            RuntimeValue::Integer(((f_arity.required - argc_offset) as i64).into());

        let arity_object = Object::new(&arity_cache.arity_struct)
            .with_value("min", lower_bound_value)
            .with_value("max", upper_bound_value)
            .with_value("has_receiver", RuntimeValue::Boolean(has_receiver.into()));

        frame.stack.push(RuntimeValue::Object(arity_object));
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity {
            required: 1,
            optional: 0,
        }
    }

    fn name(&self) -> &str {
        "arity"
    }
}

pub(super) fn insert_builtins(builtins: &mut VmGlobals) {
    builtins.insert_builtin::<Arity>();
}
