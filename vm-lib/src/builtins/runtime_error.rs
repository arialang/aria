// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::BuiltinTypeId;

use crate::runtime_value::{
    RuntimeValue,
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
    structure::Struct,
};

use super::VmGlobals;

pub(super) fn insert_runtime_error_builtins(builtins: &mut VmGlobals) {
    let argc_mismatch = Struct::new("ArgcMismatch");
    let int = builtins.get_builtin_type_by_id(BuiltinTypeId::Int);
    let str = builtins.get_builtin_type_by_id(BuiltinTypeId::String);

    let rt_err_enum = Enum::new_with_cases(
        "RuntimeError",
        &[
            EnumCase {
                name: "DivisionByZero".to_owned(),
                payload_type: None,
            },
            EnumCase {
                name: "EnumWithoutPayload".to_owned(),
                payload_type: None,
            },
            EnumCase {
                name: "IndexOutOfBounds".to_owned(),
                payload_type: Some(IsaCheckable::Type(int.clone())),
            },
            EnumCase {
                name: "MismatchedArgumentCount".to_owned(),
                payload_type: Some(IsaCheckable::Type(RuntimeValueType::Struct(
                    argc_mismatch.clone(),
                ))),
            },
            EnumCase {
                name: "NoSuchCase".to_owned(),
                payload_type: Some(IsaCheckable::Type(str.clone())),
            },
            EnumCase {
                name: "NoSuchIdentifier".to_owned(),
                payload_type: Some(IsaCheckable::Type(str.clone())),
            },
            EnumCase {
                name: "OperationFailed".to_owned(),
                payload_type: Some(IsaCheckable::Type(str.clone())),
            },
            EnumCase {
                name: "UnexpectedType".to_owned(),
                payload_type: None,
            },
        ],
    );

    let argc_sym = builtins
        .intern_symbol("ArgcMismatch")
        .expect("failed to intern builtin name");
    rt_err_enum.store_named_value(
        argc_sym,
        RuntimeValue::Type(RuntimeValueType::Struct(argc_mismatch)),
    );

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::RuntimeError,
        RuntimeValueType::Enum(rt_err_enum),
    );
}
