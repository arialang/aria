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

trait CharFunctionImpl {
    fn do_check(c: char) -> bool;
    fn name() -> &'static str;
}

macro_rules! make_char_fn {
    ($name:ident, $method:ident) => {
        #[allow(non_camel_case_types)]
        #[derive(Default)]
        struct $name {}

        impl CharFunctionImpl for $name {
            fn do_check(c: char) -> bool {
                c.$method()
            }

            fn name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

make_char_fn!(is_lowercase_letter, is_letter_lowercase);
make_char_fn!(is_uppercase_letter, is_letter_uppercase);
make_char_fn!(is_digit, is_number);
make_char_fn!(is_whitespace, is_whitespace);

struct CharBuiltinFunction<T: CharFunctionImpl + Default> {
    _marker: std::marker::PhantomData<T>,
}

impl<T: CharFunctionImpl + Default> Default for CharBuiltinFunction<T> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T: CharFunctionImpl + Default> BuiltinFunctionImpl for CharBuiltinFunction<T> {
    fn eval(
        &self,
        cur_frame: &mut haxby_vm::frame::Frame,
        _: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<RunloopExit> {
        let this = VmBuiltins::extract_arg(cur_frame, |x: RuntimeValue| x.as_string().cloned())?;
        let this_raw = this.raw_value();
        if let Some(char0) = this_raw.chars().next() {
            let is = T::do_check(char0);
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
        T::name()
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
        string.insert_builtin::<CharBuiltinFunction<is_lowercase_letter>>();
        string.insert_builtin::<CharBuiltinFunction<is_uppercase_letter>>();
        string.insert_builtin::<CharBuiltinFunction<is_digit>>();
        string.insert_builtin::<CharBuiltinFunction<is_whitespace>>();
        return LoadResult::success();
    }

    LoadResult::error("cannot inject class methods into String builtin")
}
