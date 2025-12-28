// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::BuiltinTypeId;

use crate::runtime_value::{
    RuntimeValue,
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
    structure::Struct,
};

use super::VmBuiltins;

pub(super) fn insert_runtime_error_builtins(builtins: &mut VmBuiltins) {
    let argc_mismatch = Struct::new("ArgcMismatch");

    let rt_err_enum = Enum::new("RuntimeError");

    rt_err_enum.store_named_value(
        "ArgcMismatch",
        RuntimeValue::Type(RuntimeValueType::Struct(argc_mismatch.clone())),
    );

    let int = builtins
        .get_builtin_type_by_id(BuiltinTypeId::Int)
        .expect("RuntimeError needs Int defined");
    let str = builtins
        .get_builtin_type_by_id(BuiltinTypeId::String)
        .expect("RuntimeError needs String defined");

    rt_err_enum.add_case(EnumCase {
        name: "DivisionByZero".to_owned(),
        payload_type: None,
    });
    rt_err_enum.add_case(EnumCase {
        name: "EnumWithoutPayload".to_owned(),
        payload_type: None,
    });
    rt_err_enum.add_case(EnumCase {
        name: "IndexOutOfBounds".to_owned(),
        payload_type: Some(IsaCheckable::Type(int.clone())),
    });
    rt_err_enum.add_case(EnumCase {
        name: "MismatchedArgumentCount".to_owned(),
        payload_type: Some(IsaCheckable::Type(RuntimeValueType::Struct(argc_mismatch))),
    });
    rt_err_enum.add_case(EnumCase {
        name: "NoSuchCase".to_owned(),
        payload_type: Some(IsaCheckable::Type(str.clone())),
    });
    rt_err_enum.add_case(EnumCase {
        name: "NoSuchIdentifier".to_owned(),
        payload_type: Some(IsaCheckable::Type(str.clone())),
    });
    rt_err_enum.add_case(EnumCase {
        name: "OperationFailed".to_owned(),
        payload_type: Some(IsaCheckable::Type(str.clone())),
    });
    rt_err_enum.add_case(EnumCase {
        name: "UnexpectedType".to_owned(),
        payload_type: None,
    });

    builtins.insert(
        "RuntimeError",
        RuntimeValue::Type(RuntimeValueType::Enum(rt_err_enum)),
    );
}
