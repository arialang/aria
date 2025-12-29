// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    RuntimeValue,
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

    builtins.insert(
        "Unit",
        RuntimeValue::Type(RuntimeValueType::Enum(unit_enum)),
    );
}
