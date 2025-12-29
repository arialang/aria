// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    RuntimeValue, kind::RuntimeValueType, rust_native_type::RustNativeType,
};

use super::VmGlobals;

pub(super) fn insert_boolean_builtins(builtins: &mut VmGlobals) {
    let bool_builtin =
        RustNativeType::new(crate::runtime_value::rust_native_type::RustNativeValueKind::Boolean);

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Bool,
        RuntimeValueType::RustNative(bool_builtin),
    );

    builtins.insert("true", RuntimeValue::Boolean(true.into()));
    builtins.insert("false", RuntimeValue::Boolean(false.into()));
}
