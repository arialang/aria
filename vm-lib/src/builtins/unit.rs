// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    enumeration::{Enum, EnumCase},
    kind::RuntimeValueType,
};

use super::VmGlobals;

pub(super) fn insert_unit_builtins(builtins: &mut VmGlobals) {
    let unit_enum = Enum::new("Unit");

    unit_enum.add_case(EnumCase {
        name: "unit".to_owned(),
        payload_type: None,
    });

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Unit,
        RuntimeValueType::Enum(unit_enum.clone()),
    );
}
