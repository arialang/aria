// SPDX-License-Identifier: Apache-2.0

use crate::{error::vm_error::VmErrorReason, frame::Frame, vm::VirtualMachine};

use super::{RuntimeValue, builtin_value::BuiltinValue};

pub type StringValue = BuiltinValue<String>;

impl From<&str> for StringValue {
    fn from(value: &str) -> Self {
        From::from(value.to_owned())
    }
}

impl StringValue {
    pub fn len(&self) -> usize {
        self.imp.val.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.imp.val.is_empty()
    }

    pub fn contains(&self, pat: &str) -> bool {
        self.imp.val.contains(pat)
    }
}

impl std::ops::Add<&StringValue> for &StringValue {
    type Output = StringValue;

    fn add(self, rhs: &StringValue) -> Self::Output {
        From::from(format!("{}{}", self.raw_value(), rhs.raw_value()))
    }
}

impl PartialEq<StringValue> for StringValue {
    fn eq(&self, other: &StringValue) -> bool {
        self.raw_value() == other.raw_value()
    }
}
impl Eq for StringValue {}

impl PartialOrd<StringValue> for StringValue {
    fn partial_cmp(&self, other: &StringValue) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for StringValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw_value().cmp(other.raw_value())
    }
}

impl PartialEq<str> for StringValue {
    fn eq(&self, other: &str) -> bool {
        self.raw_value() == other
    }
}

impl StringValue {
    pub fn get_at(&self, idx: usize) -> Option<RuntimeValue> {
        self.raw_value()
            .chars()
            .nth(idx)
            .map(|c| RuntimeValue::String(c.to_string().into()))
    }

    pub fn read_index(
        &self,
        idx: &RuntimeValue,
        _: &mut Frame,
        _: &mut VirtualMachine,
    ) -> Result<RuntimeValue, VmErrorReason> {
        if let Some(i) = idx.as_integer() {
            match self.get_at(*i.raw_value() as usize) {
                Some(val) => Ok(val),
                _ => Err(VmErrorReason::IndexOutOfBounds(*i.raw_value() as usize)),
            }
        } else {
            Err(VmErrorReason::UnexpectedType)
        }
    }
}
