// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{RuntimeValue, kind::RuntimeValueType, structure::Struct};

use super::VmBuiltins;

pub(super) fn insert_unimplemented_builtins(builtins: &mut VmBuiltins) {
    let unimplemented_struct = Struct::new("Unimplemented");
    builtins.insert(
        "Unimplemented",
        RuntimeValue::Type(RuntimeValueType::Struct(unimplemented_struct)),
    );
}
