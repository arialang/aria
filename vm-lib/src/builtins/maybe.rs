// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
};

use super::VmGlobals;

pub(super) fn insert_maybe_builtins(builtins: &mut VmGlobals) {
    let maybe_enum = Enum::new("Maybe");

    maybe_enum.add_case(EnumCase {
        name: "Some".to_owned(),
        payload_type: Some(IsaCheckable::any()),
    });

    maybe_enum.add_case(EnumCase {
        name: "None".to_owned(),
        payload_type: None,
    });

    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Maybe,
        RuntimeValueType::Enum(maybe_enum),
    );
}
