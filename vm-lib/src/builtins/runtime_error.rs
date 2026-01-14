// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::BuiltinTypeId;

use crate::{
    runtime_value::{
        RuntimeValue,
        enumeration::{Enum, EnumCase},
        isa::IsaCheckable,
        kind::RuntimeValueType,
        structure::Struct,
    },
    symbol::INTERNED_ATTR_ARGC_MISMATCH,
};

use super::VmGlobals;

pub const RUNTIME_ERR_CASE_DIVISION_BY_ZERO_IDX: usize = 0;
pub const RUNTIME_ERR_CASE_ENUM_WITHOUT_PAYLOAD_IDX: usize = 1;
pub const RUNTIME_ERR_CASE_INDEX_OUT_OF_BOUNDS_IDX: usize = 2;
pub const RUNTIME_ERR_CASE_MISMATCHED_ARGC_IDX: usize = 3;
pub const RUNTIME_ERR_CASE_NO_SUCH_CASE_IDX: usize = 4;
pub const RUNTIME_ERR_CASE_NO_SUCH_IDENTIFIER_IDX: usize = 5;
pub const RUNTIME_ERR_CASE_OPERATION_FAILED_IDX: usize = 6;
pub const RUNTIME_ERR_CASE_UNEXPECTED_TYPE_IDX: usize = 7;

pub(super) fn insert_runtime_error_builtins(builtins: &mut VmGlobals) {
    let argc_mismatch = Struct::new("ArgcMismatch");
    let int = builtins.get_builtin_type_by_id(BuiltinTypeId::Int);
    let str = builtins.get_builtin_type_by_id(BuiltinTypeId::String);
    let division_by_zero_sym = builtins
        .intern_symbol("DivisionByZero")
        .expect("too many symbols interned");
    let enum_without_payload_sym = builtins
        .intern_symbol("EnumWithoutPayload")
        .expect("too many symbols interned");
    let index_out_of_bounds_sym = builtins
        .intern_symbol("IndexOutOfBounds")
        .expect("too many symbols interned");
    let mismatched_argc_sym = builtins
        .intern_symbol("MismatchedArgumentCount")
        .expect("too many symbols interned");
    let no_such_case_sym = builtins
        .intern_symbol("NoSuchCase")
        .expect("too many symbols interned");
    let no_such_identifier_sym = builtins
        .intern_symbol("NoSuchIdentifier")
        .expect("too many symbols interned");
    let operation_failed_sym = builtins
        .intern_symbol("OperationFailed")
        .expect("too many symbols interned");
    let unexpected_type_sym = builtins
        .intern_symbol("UnexpectedType")
        .expect("too many symbols interned");

    let rt_err_enum = RuntimeValue::Type(RuntimeValueType::Enum(Enum::new_with_cases(
        "RuntimeError",
        &[
            EnumCase {
                name: division_by_zero_sym,
                payload_type: None,
            },
            EnumCase {
                name: enum_without_payload_sym,
                payload_type: None,
            },
            EnumCase {
                name: index_out_of_bounds_sym,
                payload_type: Some(IsaCheckable::Type(int.clone())),
            },
            EnumCase {
                name: mismatched_argc_sym,
                payload_type: Some(IsaCheckable::Type(RuntimeValueType::Struct(
                    argc_mismatch.clone(),
                ))),
            },
            EnumCase {
                name: no_such_case_sym,
                payload_type: Some(IsaCheckable::Type(str.clone())),
            },
            EnumCase {
                name: no_such_identifier_sym,
                payload_type: Some(IsaCheckable::Type(str.clone())),
            },
            EnumCase {
                name: operation_failed_sym,
                payload_type: Some(IsaCheckable::Type(str.clone())),
            },
            EnumCase {
                name: unexpected_type_sym,
                payload_type: None,
            },
        ],
        builtins,
    )));

    let _ = rt_err_enum.write_attribute(
        INTERNED_ATTR_ARGC_MISMATCH,
        RuntimeValue::Type(RuntimeValueType::Struct(argc_mismatch)),
        builtins,
    );

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::RuntimeError,
        rt_err_enum
            .as_type()
            .expect("RuntimeError is a type")
            .clone(),
    );
}
