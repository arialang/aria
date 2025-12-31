// SPDX-License-Identifier: Apache-2.0

use std::rc::Rc;

use aria_compiler::constant_value::ConstantValue;
use boolean::BooleanValue;
use bound_function::BoundFunction;
use enum_as_inner::EnumAsInner;
use enum_case::EnumValue;
use enumeration::Enum;
use float::FloatValue;
use function::Function;
use haxby_opcodes::BuiltinTypeId;
use integer::IntegerValue;
use kind::RuntimeValueType;
use list::List;
use mixin::Mixin;
use object::Object;
use opaque::OpaqueValue;
use runtime_code_object::CodeObject;
use rust_native_type::RustNativeType;
use string::StringValue;
use structure::Struct;

use crate::{
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
    frame::Frame,
    runloop::CallInvocationScheme,
    runtime_module::RuntimeModule,
    runtime_value::isa::IsaCheckable,
    vm::{ExecutionResult, VirtualMachine},
};

pub mod boolean;
pub mod bound_function;
pub mod builtin_value;
pub mod enum_case;
pub mod enumeration;
pub mod float;
pub mod function;
pub mod integer;
pub mod isa;
pub mod kind;
pub mod list;
pub mod mixin;
pub mod object;
pub mod opaque;
pub mod runtime_code_object;
pub mod rust_native_type;
pub mod string;
pub mod structure;

#[derive(EnumAsInner, Clone)]
pub enum RuntimeValue {
    Integer(IntegerValue),
    String(StringValue),
    Float(FloatValue),
    Boolean(BooleanValue),
    Object(Object),
    EnumValue(EnumValue),
    CodeObject(CodeObject),
    Function(Function),
    BoundFunction(BoundFunction),
    List(List),
    Mixin(Mixin),
    Type(RuntimeValueType),
    Module(RuntimeModule),
    Opaque(OpaqueValue),
    TypeCheck(IsaCheckable),
}

impl RuntimeValue {
    pub fn builtin_equals(
        &self,
        other: &Self,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> bool {
        match (self, other) {
            (Self::Integer(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Float(l0), Self::Float(r0)) => l0 == r0,
            (Self::Float(l0), Self::Integer(r0)) => l0 == r0,
            (Self::Integer(l0), Self::Float(r0)) => l0 == r0,
            (Self::Boolean(l0), Self::Boolean(r0)) => l0 == r0,
            (Self::String(l0), Self::String(r0)) => l0 == r0,
            (Self::Object(l0), Self::Object(r0)) => l0 == r0,
            (Self::Mixin(l0), Self::Mixin(r0)) => l0 == r0,
            (Self::Module(l0), Self::Module(r0)) => l0 == r0,
            (Self::EnumValue(l0), Self::EnumValue(r0)) => l0.builtin_equals(r0, cur_frame, vm),
            (Self::CodeObject(l0), Self::CodeObject(r0)) => l0 == r0,
            (Self::Function(l0), Self::Function(r0)) => l0 == r0,
            (Self::BoundFunction(l0), Self::BoundFunction(r0)) => l0 == r0,
            (Self::List(l0), Self::List(r0)) => l0 == r0,
            (Self::Type(l0), Self::Type(r0)) => l0 == r0,
            (Self::TypeCheck(l0), Self::TypeCheck(r0)) => l0 == r0,
            _ => false,
        }
    }
}

pub(crate) enum OperatorEvalAttemptOutcome<SuccessType> {
    Ok(SuccessType),
    Exception(crate::error::exception::VmException),
    Error(crate::error::vm_error::VmError),
    NeedTryROperator,
}

pub(crate) enum OperatorEvalOutcome<SuccessType> {
    Ok(SuccessType),
    Exception(crate::error::exception::VmException),
    Error(crate::error::vm_error::VmError),
}

impl RuntimeValue {
    pub(crate) fn is_builtin_unimplemented(&self, vm: &mut VirtualMachine) -> bool {
        if let Some(s) = self.as_object() {
            let unimp = vm
                .globals
                .get_builtin_type_by_id(BuiltinTypeId::Unimplemented);
            let unimplemented = unimp.as_struct().unwrap();
            return s.get_struct() == unimplemented;
        }

        false
    }

    fn try_eval_rel_op(
        rel_op_obj: RuntimeValue,
        other_val: &RuntimeValue,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> OperatorEvalAttemptOutcome<bool> {
        cur_frame.stack.push(other_val.clone());
        match rel_op_obj.eval(1, cur_frame, vm, true) {
            Ok(cr) => match cr {
                CallResult::OkNoValue => OperatorEvalAttemptOutcome::NeedTryROperator,
                CallResult::Ok(rv) => {
                    if let Some(bl) = rv.as_boolean() {
                        OperatorEvalAttemptOutcome::Ok(bl.raw_value())
                    } else {
                        OperatorEvalAttemptOutcome::NeedTryROperator
                    }
                }
                CallResult::Exception(e) => {
                    if e.is_builtin_unimplemented(vm) {
                        OperatorEvalAttemptOutcome::NeedTryROperator
                    } else {
                        OperatorEvalAttemptOutcome::Exception(e)
                    }
                }
            },
            Err(err) => OperatorEvalAttemptOutcome::Error(err),
        }
    }

    fn try_eval_bin_op(
        op_equals: RuntimeValue,
        other_val: &RuntimeValue,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> OperatorEvalAttemptOutcome<RuntimeValue> {
        cur_frame.stack.push(other_val.clone());
        match op_equals.eval(1, cur_frame, vm, true) {
            Ok(cr) => match cr {
                CallResult::OkNoValue => OperatorEvalAttemptOutcome::NeedTryROperator,
                CallResult::Ok(rv) => OperatorEvalAttemptOutcome::Ok(rv),
                CallResult::Exception(e) => {
                    if e.is_builtin_unimplemented(vm) {
                        OperatorEvalAttemptOutcome::NeedTryROperator
                    } else {
                        OperatorEvalAttemptOutcome::Exception(e)
                    }
                }
            },
            Err(e) => OperatorEvalAttemptOutcome::Error(e),
        }
    }

    fn try_eval_unary_op(
        op: RuntimeValue,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> OperatorEvalAttemptOutcome<RuntimeValue> {
        match op.eval(0, cur_frame, vm, true) {
            Ok(cr) => match cr {
                CallResult::OkNoValue => OperatorEvalAttemptOutcome::NeedTryROperator,
                CallResult::Ok(rv) => OperatorEvalAttemptOutcome::Ok(rv),
                CallResult::Exception(e) => {
                    if e.is_builtin_unimplemented(vm) {
                        OperatorEvalAttemptOutcome::NeedTryROperator
                    } else {
                        OperatorEvalAttemptOutcome::Exception(e)
                    }
                }
            },
            Err(e) => OperatorEvalAttemptOutcome::Error(e),
        }
    }

    pub fn equals(
        lhs: &RuntimeValue,
        rhs: &RuntimeValue,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> bool {
        if let Ok(op_equals) = lhs.read_attribute("_op_impl_equals", &vm.globals) {
            match RuntimeValue::try_eval_rel_op(op_equals, rhs, cur_frame, vm) {
                OperatorEvalAttemptOutcome::Ok(val) => {
                    return val;
                }
                OperatorEvalAttemptOutcome::Exception(_) => {
                    return lhs.builtin_equals(rhs, cur_frame, vm);
                }
                OperatorEvalAttemptOutcome::Error(_) => {
                    return lhs.builtin_equals(rhs, cur_frame, vm);
                }
                OperatorEvalAttemptOutcome::NeedTryROperator => {}
            }
        }

        if RuntimeValueType::get_type(lhs, &vm.globals)
            == RuntimeValueType::get_type(rhs, &vm.globals)
        {
            return lhs.builtin_equals(rhs, cur_frame, vm);
        }

        if let Ok(op_equals) = rhs.read_attribute("_op_impl_equals", &vm.globals) {
            return match RuntimeValue::try_eval_rel_op(op_equals, lhs, cur_frame, vm) {
                OperatorEvalAttemptOutcome::Ok(val) => val,
                OperatorEvalAttemptOutcome::Exception(_)
                | OperatorEvalAttemptOutcome::Error(_)
                | OperatorEvalAttemptOutcome::NeedTryROperator => {
                    lhs.builtin_equals(rhs, cur_frame, vm)
                }
            };
        }

        lhs.builtin_equals(rhs, cur_frame, vm)
    }
}

macro_rules! rel_op_impl {
    ($rust_fn_name: ident for $aria_fwd_name: ident rev $aria_rev_name: ident) => {
        impl RuntimeValue {
            pub(crate) fn $rust_fn_name(
                lhs: &RuntimeValue,
                rhs: &RuntimeValue,
                cur_frame: &mut Frame,
                vm: &mut VirtualMachine,
            ) -> OperatorEvalOutcome<RuntimeValue> {
                let func_name = concat!("_op_impl_", stringify!($aria_fwd_name));
                if let Ok(op) = lhs.read_attribute(func_name, &vm.globals) {
                    match RuntimeValue::try_eval_rel_op(op, rhs, cur_frame, vm) {
                        OperatorEvalAttemptOutcome::Ok(rv) => {
                            return OperatorEvalOutcome::Ok(RuntimeValue::Boolean(rv.into()));
                        }
                        OperatorEvalAttemptOutcome::Exception(e) => {
                            return OperatorEvalOutcome::Exception(e);
                        }
                        OperatorEvalAttemptOutcome::Error(e) => {
                            return OperatorEvalOutcome::Error(e);
                        }
                        OperatorEvalAttemptOutcome::NeedTryROperator => {}
                    }
                }

                if RuntimeValueType::get_type(lhs, &vm.globals)
                    == RuntimeValueType::get_type(rhs, &vm.globals)
                {
                    return OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into());
                }

                let func_name = concat!("_op_impl_", stringify!($aria_rev_name));
                if let Ok(op) = rhs.read_attribute(func_name, &vm.globals) {
                    match RuntimeValue::try_eval_rel_op(op, lhs, cur_frame, vm) {
                        OperatorEvalAttemptOutcome::Ok(rv) => {
                            return OperatorEvalOutcome::Ok(RuntimeValue::Boolean(rv.into()));
                        }
                        OperatorEvalAttemptOutcome::Exception(e) => {
                            OperatorEvalOutcome::Exception(e)
                        }
                        OperatorEvalAttemptOutcome::Error(e) => OperatorEvalOutcome::Error(e),
                        OperatorEvalAttemptOutcome::NeedTryROperator => {
                            OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into())
                        }
                    }
                } else {
                    OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into())
                }
            }
        }
    };
}

macro_rules! bin_op_impl {
    ($rust_fn_name: ident for $aria_fn_name: ident) => {
        impl RuntimeValue {
            pub(crate) fn $rust_fn_name(
                lhs: &RuntimeValue,
                rhs: &RuntimeValue,
                cur_frame: &mut Frame,
                vm: &mut VirtualMachine,
            ) -> OperatorEvalOutcome<RuntimeValue> {
                let func_name = concat!("_op_impl_", stringify!($aria_fn_name));
                if let Ok(op) = lhs.read_attribute(func_name, &vm.globals) {
                    match RuntimeValue::try_eval_bin_op(op, rhs, cur_frame, vm) {
                        OperatorEvalAttemptOutcome::Ok(rv) => {
                            return OperatorEvalOutcome::Ok(rv);
                        }
                        OperatorEvalAttemptOutcome::Exception(e) => {
                            return OperatorEvalOutcome::Exception(e);
                        }
                        OperatorEvalAttemptOutcome::Error(e) => {
                            return OperatorEvalOutcome::Error(e);
                        }
                        OperatorEvalAttemptOutcome::NeedTryROperator => {}
                    }
                }

                if RuntimeValueType::get_type(lhs, &vm.globals)
                    == RuntimeValueType::get_type(rhs, &vm.globals)
                {
                    return OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into());
                }

                let func_name = concat!("_op_impl_r", stringify!($aria_fn_name));
                if let Ok(op) = rhs.read_attribute(func_name, &vm.globals) {
                    match RuntimeValue::try_eval_bin_op(op, lhs, cur_frame, vm) {
                        OperatorEvalAttemptOutcome::Ok(rv) => OperatorEvalOutcome::Ok(rv),
                        OperatorEvalAttemptOutcome::Exception(e) => {
                            OperatorEvalOutcome::Exception(e)
                        }
                        OperatorEvalAttemptOutcome::Error(e) => OperatorEvalOutcome::Error(e),
                        OperatorEvalAttemptOutcome::NeedTryROperator => {
                            OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into())
                        }
                    }
                } else {
                    OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into())
                }
            }
        }
    };
}

macro_rules! unary_op_impl {
    ($rust_fn_name: ident for $aria_fn_name: ident) => {
        impl RuntimeValue {
            pub(crate) fn $rust_fn_name(
                obj: &RuntimeValue,
                cur_frame: &mut Frame,
                vm: &mut VirtualMachine,
            ) -> OperatorEvalOutcome<RuntimeValue> {
                if let Ok(op) =
                    obj.read_attribute(concat!("_op_impl_", stringify!($aria_fn_name)), &vm.globals)
                {
                    match RuntimeValue::try_eval_unary_op(op, cur_frame, vm) {
                        OperatorEvalAttemptOutcome::Ok(rv) => OperatorEvalOutcome::Ok(rv),
                        OperatorEvalAttemptOutcome::Exception(e) => {
                            OperatorEvalOutcome::Exception(e)
                        }
                        OperatorEvalAttemptOutcome::Error(e) => OperatorEvalOutcome::Error(e),
                        OperatorEvalAttemptOutcome::NeedTryROperator => {
                            OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into())
                        }
                    }
                } else {
                    OperatorEvalOutcome::Error(VmErrorReason::UnexpectedType.into())
                }
            }
        }
    };
}

bin_op_impl!(add for add);
bin_op_impl!(sub for sub);
bin_op_impl!(mul for mul);
bin_op_impl!(div for div);
bin_op_impl!(rem for rem);

bin_op_impl!(leftshift for lshift);
bin_op_impl!(rightshift for rshift);

bin_op_impl!(bitwise_and for bwand);
bin_op_impl!(bitwise_or for bwor);
bin_op_impl!(xor for xor);

rel_op_impl!(less_than for lt rev gt);
rel_op_impl!(greater_than for gt rev lt);

rel_op_impl!(less_than_equal for lteq rev gteq);
rel_op_impl!(greater_than_equal for gteq rev lteq);

unary_op_impl!(neg for neg);

impl std::fmt::Debug for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(x) => write!(f, "{}", x.raw_value()),
            Self::Float(x) => write!(f, "{}", x.raw_value()),
            Self::Boolean(x) => write!(f, "{}", x.raw_value()),
            Self::String(s) => write!(f, "\"{}\"", s.raw_value()),
            Self::Object(o) => write!(f, "<object of type {}>", o.get_struct().name()),
            Self::Opaque(_) => write!(f, "<opaque>"),
            Self::Mixin(m) => write!(f, "<mixin{}>", m.name()),
            Self::Module(_) => write!(f, "<module>"),
            Self::EnumValue(v) => {
                write!(f, "<enum-value of type {}>", v.get_container_enum().name())
            }
            Self::CodeObject(co) => write!(f, "{co:?}"),
            Self::Function(fnc) => write!(f, "{fnc:?}"),
            Self::BoundFunction(_) => write!(f, "<bound-function>"),
            Self::List(lt) => write!(f, "{lt:?}"),
            Self::Type(t) => write!(f, "type<{t:?}>"),
            Self::TypeCheck(t) => write!(f, "type-check({t:?})"),
        }
    }
}

impl std::fmt::Display for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeValue::String(s) => write!(f, "{}", s.raw_value()),
            _ => (self as &dyn std::fmt::Debug).fmt(f),
        }
    }
}

impl From<&ConstantValue> for RuntimeValue {
    fn from(value: &ConstantValue) -> Self {
        match value {
            ConstantValue::Integer(n) => RuntimeValue::Integer(From::from(*n)),
            ConstantValue::String(s) => RuntimeValue::String(s.to_owned().into()),
            ConstantValue::CompiledCodeObject(s) => RuntimeValue::CodeObject(From::from(s)),
            ConstantValue::Float(f) => RuntimeValue::Float(f.raw_value().into()),
        }
    }
}

pub enum AttributeError {
    NoSuchAttribute,
    InvalidFunctionBinding,
    ValueHasNoAttributes,
}

macro_rules! val_or_bound_func {
    ($val: expr, $self: expr) => {
        if let Some(rf) = $val.as_function() {
            if rf.attribute().is_type_method() {
                Err(AttributeError::InvalidFunctionBinding)
            } else {
                Ok($self.bind(rf.clone()))
            }
        } else {
            Ok($val)
        }
    };
}

pub enum CallResult<T = RuntimeValue> {
    OkNoValue,
    Ok(T),
    Exception(crate::error::exception::VmException),
}

impl RuntimeValue {
    pub fn bind(&self, f: Function) -> RuntimeValue {
        RuntimeValue::BoundFunction(BoundFunction::bind(self.clone(), f))
    }

    pub fn as_struct(&self) -> Option<&Struct> {
        self.as_type().and_then(|rt| rt.as_struct())
    }

    pub fn as_enum(&self) -> Option<&Enum> {
        self.as_type().and_then(|rt| rt.as_enum())
    }

    pub fn is_struct(&self) -> bool {
        self.as_struct().is_some()
    }

    pub fn is_enum(&self) -> bool {
        self.as_enum().is_some()
    }

    pub fn as_rust_native(&self) -> Option<&RustNativeType> {
        self.as_type().and_then(|rt| rt.as_rust_native())
    }

    pub fn as_opaque_concrete<T: 'static>(&self) -> Option<Rc<T>> {
        self.as_opaque().and_then(|c| c.as_concrete_object::<T>())
    }

    pub fn prepare_invocation(
        &self,
        argc: u8,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> ExecutionResult<CallInvocationScheme> {
        if let Some(f) = self.as_function() {
            f.prepare_invocation(argc, cur_frame, vm, &Default::default())
        } else if let Some(bf) = self.as_bound_function() {
            bf.prepare_invocation(argc, cur_frame, vm)
        } else {
            match self.read_attribute("_op_impl_call", &vm.globals) {
                Ok(op_call) => op_call.prepare_invocation(argc, cur_frame, vm),
                _ => Err(crate::error::vm_error::VmErrorReason::UnexpectedType.into()),
            }
        }
    }

    pub fn eval(
        &self,
        argc: u8,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
        discard_result: bool,
    ) -> ExecutionResult<CallResult> {
        if let Some(f) = self.as_function() {
            f.eval(argc, cur_frame, vm, &Default::default(), discard_result)
        } else if let Some(bf) = self.as_bound_function() {
            bf.eval(argc, cur_frame, vm, discard_result)
        } else {
            match self.read_attribute("_op_impl_call", &vm.globals) {
                Ok(op_call) => op_call.eval(argc, cur_frame, vm, discard_result),
                _ => Err(crate::error::vm_error::VmErrorReason::UnexpectedType.into()),
            }
        }
    }

    pub fn prettyprint(&self, cur_frame: &mut Frame, vm: &mut VirtualMachine) -> String {
        if let Ok(ppf) = self.read_attribute("prettyprint", &vm.globals)
            && ppf.eval(0, cur_frame, vm, false).is_ok()
        {
            // either check that the stack is doing ok - or have eval return the value
            if let Some(val) = cur_frame.stack.try_pop()
                && let Some(sv) = val.as_string()
            {
                return sv.raw_value().clone();
            }
        }

        format!("{self}")
    }

    pub fn write_attribute(
        &self,
        attr_name: &str,
        val: RuntimeValue,
    ) -> Result<(), AttributeError> {
        if let Some(obj) = self.as_object() {
            obj.write(attr_name, val);
            Ok(())
        } else if let Some(i) = self.as_integer() {
            i.write(attr_name, val);
            Ok(())
        } else if let Some(i) = self.as_float() {
            i.write(attr_name, val);
            Ok(())
        } else if let Some(i) = self.as_string() {
            i.write(attr_name, val);
            Ok(())
        } else if let Some(i) = self.as_boolean() {
            i.write(attr_name, val);
            Ok(())
        } else if let Some(i) = self.as_function() {
            i.write(attr_name, val);
            Ok(())
        } else if let Some(l) = self.as_list() {
            l.write(attr_name, val);
            Ok(())
        } else if let Some(t) = self.as_type() {
            t.write_attribute(attr_name, val)
        } else if let Some(m) = self.as_mixin() {
            m.store_named_value(attr_name, val);
            Ok(())
        } else if let Some(m) = self.as_module() {
            m.store_named_value(attr_name, val);
            Ok(())
        } else {
            Err(AttributeError::ValueHasNoAttributes)
        }
    }

    pub fn list_attributes(&self, builtins: &VmGlobals) -> Vec<String> {
        if let Some(obj) = self.as_object() {
            let mut attrs = obj.list_attributes();
            attrs.extend(obj.get_struct().list_attributes());
            attrs.iter().cloned().collect()
        } else if let Some(mixin) = self.as_mixin() {
            mixin.list_attributes().iter().cloned().collect()
        } else if let Some(enumm) = self.as_enum_value() {
            enumm
                .get_container_enum()
                .list_attributes()
                .iter()
                .cloned()
                .collect()
        } else if let Some(i) = self.as_integer() {
            let mut attrs = i.list_attributes();
            let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::Int);
            attrs.extend(bt.list_attributes());
            attrs.iter().cloned().collect()
        } else if let Some(i) = self.as_float() {
            let mut attrs = i.list_attributes();
            let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::Float);
            attrs.extend(bt.list_attributes());
            attrs.iter().cloned().collect()
        } else if let Some(s) = self.as_string() {
            let mut attrs = s.list_attributes();
            let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::String);
            attrs.extend(bt.list_attributes());
            attrs.iter().cloned().collect()
        } else if let Some(b) = self.as_boolean() {
            let mut attrs = b.list_attributes();
            let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::Bool);
            attrs.extend(bt.list_attributes());
            attrs.iter().cloned().collect()
        } else if let Some(l) = self.as_list() {
            let mut attrs = l.list_attributes();
            let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::List);
            attrs.extend(bt.list_attributes());
            attrs.iter().cloned().collect()
        } else if let Some(f) = self.as_function() {
            f.list_attributes().iter().cloned().collect()
        } else if let Some(m) = self.as_module() {
            m.list_named_values().iter().cloned().collect()
        } else {
            vec![]
        }
    }

    pub fn read_attribute(
        &self,
        attrib_name: &str,
        builtins: &VmGlobals,
    ) -> Result<RuntimeValue, AttributeError> {
        if let Some(obj) = self.as_object() {
            match obj.read(attrib_name) {
                Some(val) => Ok(val),
                _ => match obj.get_struct().load_named_value(attrib_name) {
                    Some(val) => {
                        val_or_bound_func!(val, self)
                    }
                    _ => Err(AttributeError::NoSuchAttribute),
                },
            }
        } else if let Some(mixin) = self.as_mixin() {
            match mixin.load_named_value(attrib_name) {
                Some(val) => Ok(val),
                _ => Err(AttributeError::NoSuchAttribute),
            }
        } else if let Some(enumm) = self.as_enum_value() {
            match enumm.read(attrib_name) {
                Some(val) => {
                    val_or_bound_func!(val, self)
                }
                _ => Err(AttributeError::NoSuchAttribute),
            }
        } else if let Some(i) = self.as_integer() {
            match i.read(attrib_name) {
                Some(val) => Ok(val),
                _ => {
                    let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::Int);
                    match bt.read_attribute(attrib_name) {
                        Ok(val) => {
                            val_or_bound_func!(val, self)
                        }
                        _ => Err(AttributeError::NoSuchAttribute),
                    }
                }
            }
        } else if let Some(i) = self.as_float() {
            match i.read(attrib_name) {
                Some(val) => Ok(val),
                _ => {
                    let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::Float);
                    match bt.read_attribute(attrib_name) {
                        Ok(val) => {
                            val_or_bound_func!(val, self)
                        }
                        _ => Err(AttributeError::NoSuchAttribute),
                    }
                }
            }
        } else if let Some(f) = self.as_function() {
            match f.read(attrib_name) {
                Some(val) => Ok(val),
                _ => Err(AttributeError::NoSuchAttribute),
            }
        } else if let Some(l) = self.as_list() {
            match l.read(attrib_name) {
                Some(val) => Ok(val),
                _ => {
                    let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::List);
                    match bt.read_attribute(attrib_name) {
                        Ok(val) => {
                            val_or_bound_func!(val, self)
                        }
                        _ => Err(AttributeError::NoSuchAttribute),
                    }
                }
            }
        } else if let Some(i) = self.as_string() {
            match i.read(attrib_name) {
                Some(val) => Ok(val),
                _ => {
                    let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::String);
                    match bt.read_attribute(attrib_name) {
                        Ok(val) => {
                            val_or_bound_func!(val, self)
                        }
                        _ => Err(AttributeError::NoSuchAttribute),
                    }
                }
            }
        } else if let Some(i) = self.as_boolean() {
            match i.read(attrib_name) {
                Some(val) => Ok(val),
                _ => {
                    let bt = builtins.get_builtin_type_by_id(BuiltinTypeId::Bool);
                    match bt.read_attribute(attrib_name) {
                        Ok(val) => {
                            val_or_bound_func!(val, self)
                        }
                        _ => Err(AttributeError::NoSuchAttribute),
                    }
                }
            }
        } else if let Some(t) = self.as_type() {
            let val = t.read_attribute(attrib_name)?;
            if let Some(rf) = val.as_function() {
                if !rf.attribute().is_type_method() {
                    Err(AttributeError::InvalidFunctionBinding)
                } else {
                    Ok(self.bind(rf.clone()))
                }
            } else {
                Ok(val)
            }
        } else if let Some(m) = self.as_module() {
            match m.load_named_value(attrib_name) {
                Some(v) => Ok(v),
                None => Err(AttributeError::NoSuchAttribute),
            }
        } else {
            Err(AttributeError::ValueHasNoAttributes)
        }
    }

    pub fn read_index(
        &self,
        indices: &[RuntimeValue],
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> ExecutionResult<CallResult> {
        match self.read_attribute("_op_impl_read_index", &vm.globals) {
            Ok(read_index) => {
                for idx in indices.iter().rev() {
                    cur_frame.stack.push(idx.clone());
                }
                read_index.eval(indices.len() as u8, cur_frame, vm, false)
            }
            _ => Err(VmErrorReason::UnexpectedType.into()),
        }
    }

    pub fn write_index(
        &self,
        indices: &[RuntimeValue],
        val: &RuntimeValue,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> ExecutionResult<CallResult> {
        match self.read_attribute("_op_impl_write_index", &vm.globals) {
            Ok(write_index) => {
                cur_frame.stack.push(val.clone());
                for idx in indices.iter().rev() {
                    cur_frame.stack.push(idx.clone());
                }
                write_index.eval(1 + indices.len() as u8, cur_frame, vm, true)
            }
            _ => Err(VmErrorReason::UnexpectedType.into()),
        }
    }
}
