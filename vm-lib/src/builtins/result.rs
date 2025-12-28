// SPDX-License-Identifier: Apache-2.0
use crate::runtime_value::{
    RuntimeValue,
    enumeration::{Enum, EnumCase},
    isa::IsaCheckable,
    kind::RuntimeValueType,
};

use super::VmBuiltins;

pub(super) fn insert_result_builtins(builtins: &mut VmBuiltins) {
    let result_enum = Enum::new("Result");

    result_enum.add_case(EnumCase {
        name: "Ok".to_owned(),
        payload_type: Some(IsaCheckable::any()),
    });

    result_enum.add_case(EnumCase {
        name: "Err".to_owned(),
        payload_type: Some(IsaCheckable::any()),
    });

    builtins.insert(
        "Result",
        RuntimeValue::Type(RuntimeValueType::Enum(result_enum)),
    );
}
