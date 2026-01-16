// SPDX-License-Identifier: Apache-2.0
use super::builtin_value::BuiltinValue;

pub type BooleanValue = BuiltinValue<bool>;

impl PartialEq<BooleanValue> for BooleanValue {
    fn eq(&self, other: &BooleanValue) -> bool {
        self.raw_value() == other.raw_value()
    }
}
impl Eq for BooleanValue {}

impl std::ops::Not for BooleanValue {
    type Output = BooleanValue;

    fn not(self) -> Self::Output {
        self.raw_value().not().into()
    }
}

impl std::ops::BitXor for &BooleanValue {
    type Output = BooleanValue;

    fn bitxor(self, rhs: Self) -> Self::Output {
        (self.raw_value() ^ rhs.raw_value()).into()
    }
}

impl std::ops::BitAnd for &BooleanValue {
    type Output = BooleanValue;

    fn bitand(self, rhs: Self) -> Self::Output {
        (*self.raw_value() && *rhs.raw_value()).into()
    }
}

impl std::ops::BitOr for &BooleanValue {
    type Output = BooleanValue;

    fn bitor(self, rhs: Self) -> Self::Output {
        (*self.raw_value() || *rhs.raw_value()).into()
    }
}
