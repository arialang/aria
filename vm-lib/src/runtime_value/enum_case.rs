// SPDX-License-Identifier: Apache-2.0

use std::rc::Rc;

use crate::{frame::Frame, symbol::Symbol, vm::VirtualMachine};

use super::{RuntimeValue, enumeration::Enum};

pub(super) struct EnumValueImpl {
    pub(super) enumm: Enum,
    pub(super) case: usize,
    pub(super) payload: Option<RuntimeValue>,
}

#[derive(Clone)]
pub struct EnumValue {
    pub(super) imp: Rc<EnumValueImpl>,
}

impl EnumValue {
    pub fn get_container_enum(&self) -> &Enum {
        &self.imp.enumm
    }

    pub fn get_case_index(&self) -> usize {
        self.imp.case
    }

    pub fn get_payload(&self) -> Option<&RuntimeValue> {
        self.imp.payload.as_ref()
    }

    pub fn read(&self, name: Symbol) -> Option<RuntimeValue> {
        self.imp.enumm.load_named_value(name)
    }
}

impl EnumValueImpl {
    fn builtin_equals(&self, other: &Self, cur_frame: &mut Frame, vm: &mut VirtualMachine) -> bool {
        self.enumm == other.enumm
            && self.case == other.case
            && match (&self.payload, &other.payload) {
                (None, None) => true,
                (None, Some(_)) => false,
                (Some(_), None) => false,
                (Some(a), Some(b)) => RuntimeValue::equals(a, b, cur_frame, vm),
            }
    }
}

impl EnumValue {
    pub(super) fn builtin_equals(
        &self,
        other: &Self,
        cur_frame: &mut Frame,
        vm: &mut VirtualMachine,
    ) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp) || self.imp.builtin_equals(&other.imp, cur_frame, vm)
    }
}
