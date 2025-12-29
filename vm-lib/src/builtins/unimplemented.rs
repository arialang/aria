// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{kind::RuntimeValueType, structure::Struct};

use super::VmGlobals;

pub(super) fn insert_unimplemented_builtins(builtins: &mut VmGlobals) {
    let unimplemented_struct = Struct::new("Unimplemented");
    builtins.register_builtin_type(
        haxby_opcodes::BuiltinTypeId::Unimplemented,
        RuntimeValueType::Struct(unimplemented_struct.clone()),
    );
}
