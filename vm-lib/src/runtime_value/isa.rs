// SPDX-License-Identifier: Apache-2.0
use crate::builtins::VmGlobals;
use crate::runtime_value::mixin::Mixin;
use crate::runtime_value::{RuntimeValue, RuntimeValueType};

#[derive(Clone, PartialEq, Eq)]
pub enum IsaCheckable {
    Type(RuntimeValueType),
    Mixin(Mixin),
    Union(Vec<IsaCheckable>),
    Intersection(Vec<IsaCheckable>),
}

impl std::fmt::Debug for IsaCheckable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IsaCheckable::Type(t) => write!(f, "({:?})", t),
            IsaCheckable::Mixin(m) => write!(f, "<mixin{}>", m.name()),
            IsaCheckable::Union(us) => {
                write!(f, "(")?;
                for (i, u) in us.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{:?}", u)?;
                }
                write!(f, ")")
            }
            IsaCheckable::Intersection(is) => {
                write!(f, "(")?;
                for (i, u) in is.iter().enumerate() {
                    if i > 0 {
                        write!(f, " & ")?;
                    }
                    write!(f, "{:?}", u)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl IsaCheckable {
    fn isa(val: &RuntimeValue, t: &RuntimeValueType, builtins: &VmGlobals) -> bool {
        match t {
            RuntimeValueType::Any => true,
            RuntimeValueType::Union(u) => {
                for u_k in u {
                    if Self::isa(val, u_k, builtins) {
                        return true;
                    }
                }
                false
            }
            _ => RuntimeValueType::get_type(val, builtins) == *t,
        }
    }

    fn isa_mixin(val: &RuntimeValue, mixin: &Mixin, builtins: &VmGlobals) -> bool {
        if let Some(obj) = val.as_object() {
            obj.get_struct().isa_mixin(mixin)
        } else if let Some(env) = val.as_enum_value() {
            env.get_container_enum().isa_mixin(mixin)
        } else if let Some(m) = val.as_mixin() {
            m.isa_mixin(mixin)
        } else if let Some(st) = val.as_struct() {
            st.isa_mixin(mixin)
        } else if let Some(en) = val.as_enum() {
            en.isa_mixin(mixin)
        } else if let Some(btt) = RuntimeValueType::get_type(val, builtins).as_rust_native() {
            btt.isa_mixin(mixin)
        } else {
            false
        }
    }
}

impl IsaCheckable {
    pub fn isa_check(&self, other: &RuntimeValue, builtins: &VmGlobals) -> bool {
        match self {
            IsaCheckable::Type(t) => IsaCheckable::isa(other, t, builtins),
            IsaCheckable::Mixin(m) => IsaCheckable::isa_mixin(other, m, builtins),
            IsaCheckable::Union(us) => us.iter().any(|u| u.isa_check(other, builtins)),
            IsaCheckable::Intersection(is) => is.iter().all(|i| i.isa_check(other, builtins)),
        }
    }

    pub fn any() -> Self {
        IsaCheckable::Type(RuntimeValueType::Any)
    }
}

impl TryFrom<&RuntimeValue> for IsaCheckable {
    type Error = ();

    fn try_from(value: &RuntimeValue) -> Result<Self, Self::Error> {
        if let Some(mixin) = value.as_mixin() {
            Ok(IsaCheckable::Mixin(mixin.clone()))
        } else if let Some(t) = value.as_type() {
            Ok(IsaCheckable::Type(t.clone()))
        } else if let Some(c) = value.as_type_check() {
            Ok(c.clone())
        } else {
            Err(())
        }
    }
}

impl std::ops::BitOr<&IsaCheckable> for IsaCheckable {
    type Output = IsaCheckable;

    fn bitor(self, rhs: &IsaCheckable) -> Self::Output {
        match (self, rhs) {
            (IsaCheckable::Union(xs), IsaCheckable::Union(ys)) => {
                let mut combined = xs;
                for y in ys {
                    if !combined.contains(y) {
                        combined.push(y.clone());
                    }
                }
                IsaCheckable::Union(combined)
            }

            (IsaCheckable::Union(xs), y) => {
                let mut combined = xs;
                if !combined.contains(y) {
                    combined.push(y.clone());
                }
                IsaCheckable::Union(combined)
            }

            (x, IsaCheckable::Union(ys)) => {
                let mut combined = ys.clone();
                if !combined.contains(&x) {
                    combined.push(x);
                }
                IsaCheckable::Union(combined)
            }

            (x, y) => {
                if x == *y {
                    x
                } else {
                    IsaCheckable::Union(vec![x, y.clone()])
                }
            }
        }
    }
}

impl std::ops::BitAnd<&IsaCheckable> for IsaCheckable {
    type Output = IsaCheckable;

    fn bitand(self, rhs: &IsaCheckable) -> Self::Output {
        match (self, rhs) {
            (IsaCheckable::Intersection(xs), IsaCheckable::Intersection(ys)) => {
                let mut combined = xs;
                for y in ys {
                    if !combined.contains(y) {
                        combined.push(y.clone());
                    }
                }
                IsaCheckable::Intersection(combined)
            }

            (IsaCheckable::Intersection(xs), y) => {
                let mut combined = xs;
                if !combined.contains(y) {
                    combined.push(y.clone());
                }
                IsaCheckable::Intersection(combined)
            }

            (x, IsaCheckable::Intersection(ys)) => {
                let mut combined = ys.clone();
                if !combined.contains(&x) {
                    combined.push(x);
                }
                IsaCheckable::Intersection(combined)
            }

            (x, y) => {
                if x == *y {
                    x
                } else {
                    IsaCheckable::Intersection(vec![x, y.clone()])
                }
            }
        }
    }
}
