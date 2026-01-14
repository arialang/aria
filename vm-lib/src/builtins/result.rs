// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
};

use super::VmGlobals;

pub(super) fn insert_result_builtins(builtins: &mut VmGlobals) {
    let ok_sym = builtins
        .intern_symbol("Ok")
        .expect("too many symbols interned");
    let err_sym = builtins
        .intern_symbol("Err")
        .expect("too many symbols interned");
    let result_enum = Enum::new_with_cases(
        "Result",
        &[
            EnumCase {
                name: ok_sym,
                payload_type: Some(IsaCheckable::any()),
            },
            EnumCase {
                name: err_sym,
                payload_type: Some(IsaCheckable::any()),
            },
        ],
        builtins,
    );

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Result,
        RuntimeValueType::Enum(result_enum),
    );
}
