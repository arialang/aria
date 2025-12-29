// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{RuntimeValue, kind::RuntimeValueType, structure::Struct};

use super::VmGlobals;

pub(super) fn insert_unimplemented_builtins(builtins: &mut VmGlobals) {
    let unimplemented_struct = Struct::new("Unimplemented");
    builtins.insert(
        "Unimplemented",
        RuntimeValue::Type(RuntimeValueType::Struct(unimplemented_struct)),
    );
}
