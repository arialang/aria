// SPDX-License-Identifier: Apache-2.0

use haxby_opcodes::builtin_type_ids::BUILTIN_TYPE_STRING;
use haxby_vm::{
    builtins::VmBuiltins,
    error::dylib_load::LoadResult,
    runtime_module::RuntimeModule,
    runtime_value::{RuntimeValue, function::BuiltinFunctionImpl},
    vm::RunloopExit,
};

use unicode_categories::UnicodeCategories;

#[derive(Default)]
struct IsLowercaseLetter {}
impl BuiltinFunctionImpl for IsLowercaseLetter {
    fn eval(
        &self,
        cur_frame: &mut haxby_vm::frame::Frame,
        _: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<RunloopExit> {
        let this = VmBuiltins::extract_arg(cur_frame, |x: RuntimeValue| x.as_string().cloned())?;
        let this_raw = this.raw_value();
        if let Some(char0) = this_raw.chars().next() {
            let is = char0.is_letter_lowercase();
            cur_frame.stack.push(RuntimeValue::Boolean(is.into()));
        } else {
            todo!() // index out of bounds error
        }
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_lowercase_letter"
    }
}

#[derive(Default)]
struct IsUppercaseLetter {}
impl BuiltinFunctionImpl for IsUppercaseLetter {
    fn eval(
        &self,
        cur_frame: &mut haxby_vm::frame::Frame,
        _: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<RunloopExit> {
        let this = VmBuiltins::extract_arg(cur_frame, |x: RuntimeValue| x.as_string().cloned())?;
        let this_raw = this.raw_value();
        if let Some(char0) = this_raw.chars().next() {
            let is = char0.is_letter_uppercase();
            cur_frame.stack.push(RuntimeValue::Boolean(is.into()));
        } else {
            todo!() // index out of bounds error
        }
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_uppercase_letter"
    }
}

#[derive(Default)]
struct IsDigit {}
impl BuiltinFunctionImpl for IsDigit {
    fn eval(
        &self,
        cur_frame: &mut haxby_vm::frame::Frame,
        _: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<RunloopExit> {
        let this = VmBuiltins::extract_arg(cur_frame, |x: RuntimeValue| x.as_string().cloned())?;
        let this_raw = this.raw_value();
        if let Some(char0) = this_raw.chars().next() {
            let is = char0.is_number();
            cur_frame.stack.push(RuntimeValue::Boolean(is.into()));
        } else {
            todo!() // index out of bounds error
        }
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_digit"
    }
}

#[derive(Default)]
struct IsWhitespace {}
impl BuiltinFunctionImpl for IsWhitespace {
    fn eval(
        &self,
        cur_frame: &mut haxby_vm::frame::Frame,
        _: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<RunloopExit> {
        let this = VmBuiltins::extract_arg(cur_frame, |x: RuntimeValue| x.as_string().cloned())?;
        let this_raw = this.raw_value();
        if let Some(char0) = this_raw.chars().next() {
            let is = char0.is_whitespace();
            cur_frame.stack.push(RuntimeValue::Boolean(is.into()));
        } else {
            todo!() // index out of bounds error
        }
        Ok(RunloopExit::Ok(()))
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_whitespace"
    }
}
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn dylib_haxby_inject(
    vm: *const haxby_vm::vm::VirtualMachine,
    module: *const RuntimeModule,
) -> LoadResult {
    // the aria module isn't really useful here since it's just a placeholder to inject methods
    // into the String builtin type, but I'd rather be safe here and check that it's also a valid
    // module, not just a valid VM instance
    if let (Some(vm), Some(_)) = unsafe { (vm.as_ref(), module.as_ref()) }
        && let Some(string) = vm.builtins.get_builtin_type_by_id(BUILTIN_TYPE_STRING)
        && let Some(string) = string.as_builtin()
    {
        string.insert_builtin::<IsLowercaseLetter>();
        string.insert_builtin::<IsUppercaseLetter>();
        string.insert_builtin::<IsDigit>();
        string.insert_builtin::<IsWhitespace>();
        return LoadResult::success();
    }

    LoadResult::error("cannot inject class methods into String builtin")
}
