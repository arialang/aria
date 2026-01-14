// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    enumeration::{Enum, EnumCase},
    kind::RuntimeValueType,
};

use super::VmGlobals;

pub(super) fn insert_unit_builtins(builtins: &mut VmGlobals) {
    let unit_sym = builtins
        .intern_symbol("unit")
        .expect("too many symbols interned");
    let unit_enum = Enum::new_with_cases(
        "Unit",
        &[EnumCase {
            name: unit_sym,
            payload_type: None,
        }],
        builtins,
    );

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Unit,
        RuntimeValueType::Enum(unit_enum.clone()),
    );
}
