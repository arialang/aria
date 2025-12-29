// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{kind::RuntimeValueType, rust_native_type::RustNativeType};

use super::VmGlobals;

pub(super) fn insert_type_builtins(builtins: &mut VmGlobals) {
    let type_builtin =
        RustNativeType::new(crate::runtime_value::rust_native_type::RustNativeValueKind::Type);

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Type,
        RuntimeValueType::RustNative(type_builtin),
    );
}
