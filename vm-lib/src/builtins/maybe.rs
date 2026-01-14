// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
};

use super::VmGlobals;

pub(super) fn insert_maybe_builtins(builtins: &mut VmGlobals) {
    let some_sym = builtins
        .intern_symbol("Some")
        .expect("too many symbols interned");
    let none_sym = builtins
        .intern_symbol("None")
        .expect("too many symbols interned");
    let maybe_enum = Enum::new_with_cases(
        "Maybe",
        &[
            EnumCase {
                name: some_sym,
                payload_type: Some(IsaCheckable::any()),
            },
            EnumCase {
                name: none_sym,
                payload_type: None,
            },
        ],
        builtins,
    );

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Maybe,
        RuntimeValueType::Enum(maybe_enum),
    );
}
