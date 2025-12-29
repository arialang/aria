// SPDX-License-Identifier: Apache-2.0

use haxby_opcodes::function_attribs::FUNC_IS_METHOD;

use crate::{
    frame::Frame,
    runtime_value::{
        RuntimeValue, function::BuiltinFunctionImpl, kind::RuntimeValueType,
        rust_native_type::RustNativeType,
    },
    vm::RunloopExit,
};

use super::VmGlobals;

fn int_format(n: i64, fmt: &str) -> String {
    // Determine if format ends with 'x' or 'X' for hexadecimal formatting
    let (base, digits_spec) = if let Some(stripped) = fmt.strip_suffix('x') {
        (16, (stripped, false)) // lowercase hex
    } else if let Some(stripped) = fmt.strip_suffix('X') {
        (16, (stripped, true)) // uppercase hex
    } else {
        (10, (fmt, false)) // decimal
    };

    // Parse width from digits (if any), default to 0
    let width = digits_spec.0.parse::<usize>().unwrap_or(0);
    let uppercase = digits_spec.1;

    if base == 16 {
        // Format hex with padding and case
        if uppercase {
            format!("{n:0width$X}")
        } else {
            format!("{n:0width$x}")
        }
    } else {
        // Format decimal with padding
        format!("{n:0width$}")
    }
}

#[derive(Default)]
struct Prettyprint {}
impl BuiltinFunctionImpl for Prettyprint {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_integer().cloned())?.raw_value();
        let format_style = VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?.raw_value();
        let output_string = int_format(this, &format_style);
        frame.stack.push(RuntimeValue::String(output_string.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "prettyprint"
    }
}

pub(super) fn insert_integer_builtins(builtins: &mut VmGlobals) {
    let int_builtin =
        RustNativeType::new(crate::runtime_value::rust_native_type::RustNativeValueKind::Integer);

    int_builtin.insert_builtin::<Prettyprint>();

    builtins.insert(
        "Int",
        RuntimeValue::Type(RuntimeValueType::RustNative(int_builtin)),
    );
}
