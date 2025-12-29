// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
};

use super::VmGlobals;

pub(super) fn insert_result_builtins(builtins: &mut VmGlobals) {
    let result_enum = Enum::new("Result");

    result_enum.add_case(EnumCase {
        name: "Ok".to_owned(),
        payload_type: Some(IsaCheckable::any()),
    });

    result_enum.add_case(EnumCase {
        name: "Err".to_owned(),
        payload_type: Some(IsaCheckable::any()),
    });

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Result,
        RuntimeValueType::Enum(result_enum),
    );
}
