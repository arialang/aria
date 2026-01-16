// SPDX-License-Identifier: Apache-2.0
use super::{builtin_value::BuiltinValue, integer::IntegerValue};

pub type FloatValue = BuiltinValue<f64>;

impl PartialEq<IntegerValue> for FloatValue {
    fn eq(&self, other: &IntegerValue) -> bool {
        self.raw_value() == other.to_fp().raw_value()
    }
}

impl std::ops::Add<&FloatValue> for &FloatValue {
    type Output = FloatValue;

    fn add(self, rhs: &FloatValue) -> Self::Output {
        From::from(self.raw_value() + rhs.raw_value())
    }
}

impl std::ops::Sub<&FloatValue> for &FloatValue {
    type Output = FloatValue;

    fn sub(self, rhs: &FloatValue) -> Self::Output {
        From::from(self.raw_value() - rhs.raw_value())
    }
}

impl std::ops::Mul<&FloatValue> for &FloatValue {
    type Output = FloatValue;

    fn mul(self, rhs: &FloatValue) -> Self::Output {
        From::from(self.raw_value() * rhs.raw_value())
    }
}

impl std::ops::Div<&FloatValue> for &FloatValue {
    type Output = FloatValue;

    fn div(self, rhs: &FloatValue) -> Self::Output {
        From::from(self.raw_value() / rhs.raw_value())
    }
}

impl std::ops::Rem<&FloatValue> for &FloatValue {
    type Output = FloatValue;

    fn rem(self, rhs: &FloatValue) -> Self::Output {
        From::from(self.raw_value() % rhs.raw_value())
    }
}

impl std::ops::Neg for &FloatValue {
    type Output = FloatValue;

    fn neg(self) -> Self::Output {
        From::from(-self.raw_value())
    }
}

impl PartialEq<FloatValue> for FloatValue {
    fn eq(&self, other: &FloatValue) -> bool {
        self.raw_value() == other.raw_value()
    }
}
impl Eq for FloatValue {}

impl PartialOrd<FloatValue> for FloatValue {
    fn partial_cmp(&self, other: &FloatValue) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for FloatValue {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw_value()
            .partial_cmp(other.raw_value())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}
