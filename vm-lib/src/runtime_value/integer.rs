// SPDX-License-Identifier: Apache-2.0
use super::{builtin_value::BuiltinValue, float::FloatValue};

pub type IntegerValue = BuiltinValue<i64>;

impl IntegerValue {
    pub fn to_fp(&self) -> FloatValue {
        (*self.raw_value() as f64).into()
    }
}

impl PartialEq<FloatValue> for IntegerValue {
    fn eq(&self, other: &FloatValue) -> bool {
        self.to_fp() == *other
    }
}

impl std::ops::Add<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn add(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value().wrapping_add(*rhs.raw_value()))
    }
}

impl std::ops::Sub<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn sub(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value().wrapping_sub(*rhs.raw_value()))
    }
}

impl std::ops::Mul<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn mul(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value().wrapping_mul(*rhs.raw_value()))
    }
}

impl std::ops::Div<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn div(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value().wrapping_div(*rhs.raw_value()))
    }
}

impl std::ops::Rem<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn rem(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value().wrapping_rem(*rhs.raw_value()))
    }
}

impl std::ops::Neg for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn neg(self) -> Self::Output {
        From::from(-self.raw_value())
    }
}

impl std::ops::BitAnd<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn bitand(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value() & rhs.raw_value())
    }
}

impl std::ops::BitOr<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn bitor(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value() | rhs.raw_value())
    }
}

impl std::ops::BitXor<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn bitxor(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value() ^ rhs.raw_value())
    }
}

impl std::ops::Shl<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn shl(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value() << rhs.raw_value())
    }
}

impl std::ops::Shr<&IntegerValue> for &IntegerValue {
    type Output = IntegerValue;

    #[inline]
    fn shr(self, rhs: &IntegerValue) -> Self::Output {
        From::from(self.raw_value() >> rhs.raw_value())
    }
}

impl PartialEq<IntegerValue> for IntegerValue {
    #[inline]
    fn eq(&self, other: &IntegerValue) -> bool {
        self.raw_value() == other.raw_value()
    }
}
impl Eq for IntegerValue {}

impl PartialOrd<IntegerValue> for IntegerValue {
    #[inline]
    fn partial_cmp(&self, other: &IntegerValue) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for IntegerValue {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.raw_value().cmp(other.raw_value())
    }
}

impl PartialEq<i64> for IntegerValue {
    #[inline]
    fn eq(&self, other: &i64) -> bool {
        *self.raw_value() == *other
    }
}
