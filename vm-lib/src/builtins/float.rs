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

#[derive(Default)]
struct FpHash {}
impl BuiltinFunctionImpl for FpHash {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_float().cloned())?;
        let hv = unsafe { std::mem::transmute_copy::<f64, i64>(&this.raw_value()) };
        frame.stack.push(RuntimeValue::Integer(hv.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "hash"
    }
}

#[derive(Default)]
struct FpFloor {}
impl BuiltinFunctionImpl for FpFloor {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_float().cloned())?;
        let result = RuntimeValue::Float(this.raw_value().floor().into());
        frame.stack.push(result);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "floor"
    }
}

#[derive(Default)]
struct FpCeil {}
impl BuiltinFunctionImpl for FpCeil {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_float().cloned())?;
        let result = RuntimeValue::Float(this.raw_value().ceil().into());
        frame.stack.push(result);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "ceil"
    }
}

#[derive(Default)]
struct FpInt {}
impl BuiltinFunctionImpl for FpInt {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_float().cloned())?;
        let iv = this.raw_value() as i64;
        frame.stack.push(RuntimeValue::Integer(iv.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "int"
    }
}

#[derive(Default)]
struct FpPrettyprint {}
impl BuiltinFunctionImpl for FpPrettyprint {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let this = VmGlobals::extract_arg(frame, |x| x.as_float().cloned())?;
        let format_opt = if !frame.stack.is_empty() {
            Some(VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?)
        } else {
            None
        };

        let result = if let Some(format_str) = format_opt {
            let format = format_str.raw_value();
            if format == ".E" || format == ".e" {
                format!("{:e}", this.raw_value())
            } else if format.starts_with('.') && format.len() > 1 {
                if let Ok(precision) = format[1..].parse::<usize>() {
                    format!("{:.precision$}", this.raw_value(), precision = precision)
                } else {
                    this.raw_value().to_string()
                }
            } else {
                this.raw_value().to_string()
            }
        } else {
            this.raw_value().to_string()
        };

        frame.stack.push(RuntimeValue::String(result.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> crate::arity::Arity {
        crate::arity::Arity {
            required: 1,
            optional: 1,
        }
    }

    fn name(&self) -> &str {
        "prettyprint"
    }
}

pub(super) fn insert_float_builtins(builtins: &mut VmGlobals) {
    let fp_builtin =
        RustNativeType::new(crate::runtime_value::rust_native_type::RustNativeValueKind::Float);

    fp_builtin.insert_builtin::<FpHash>();
    fp_builtin.insert_builtin::<FpFloor>();
    fp_builtin.insert_builtin::<FpCeil>();
    fp_builtin.insert_builtin::<FpInt>();
    fp_builtin.insert_builtin::<FpPrettyprint>();

    fp_builtin.write("inf", RuntimeValue::Float(f64::INFINITY.into()));
    fp_builtin.write("nan", RuntimeValue::Float(f64::NAN.into()));
    fp_builtin.write("epsilon", RuntimeValue::Float(f64::EPSILON.into()));

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Float,
        RuntimeValueType::RustNative(fp_builtin),
    );
}
