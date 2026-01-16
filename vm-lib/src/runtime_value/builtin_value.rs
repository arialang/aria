// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use haxby_opcodes::BuiltinTypeId;
use rustc_data_structures::fx::FxHashSet;

use crate::{builtins::VmGlobals, symbol::Symbol};

use super::object::ObjectBox;

pub(crate) struct BuiltinValueImpl<T>
where
    T: Clone,
{
    pub(crate) val: T,
    id: BuiltinTypeId,
    pub(crate) boxx: ObjectBox,
}

impl<T> BuiltinValueImpl<T>
where
    T: Clone,
{
    fn list_attributes(&self, builtins: &VmGlobals) -> FxHashSet<Symbol> {
        self.boxx.list_attributes(builtins)
    }
}

#[derive(Clone)]
pub struct BuiltinValue<T>
where
    T: Clone,
{
    pub(crate) imp: Rc<BuiltinValueImpl<T>>,
}

trait GetBuiltinTypeId {
    fn get_builtin_type_id() -> BuiltinTypeId;
}

impl GetBuiltinTypeId for i64 {
    #[inline]
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::Int
    }
}
impl GetBuiltinTypeId for bool {
    #[inline]
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::Bool
    }
}
impl GetBuiltinTypeId for String {
    #[inline]
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::String
    }
}
impl GetBuiltinTypeId for f64 {
    #[inline]
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::Float
    }
}

impl<T> From<T> for BuiltinValueImpl<T>
where
    T: Clone + GetBuiltinTypeId,
{
    #[inline]
    fn from(val: T) -> Self {
        Self {
            val,
            id: T::get_builtin_type_id(),
            boxx: Default::default(),
        }
    }
}

impl<T> From<T> for BuiltinValue<T>
where
    T: Clone + GetBuiltinTypeId,
{
    #[inline]
    fn from(val: T) -> Self {
        Self {
            imp: Rc::new(From::from(val)),
        }
    }
}

impl<T> BuiltinValue<T>
where
    T: Clone,
{
    #[inline]
    pub fn builtin_type_id(&self) -> BuiltinTypeId {
        self.imp.id
    }

    #[inline]
    pub fn raw_value(&self) -> &T {
        &self.imp.val
    }

    pub fn list_attributes(&self, builtins: &VmGlobals) -> FxHashSet<Symbol> {
        self.imp.list_attributes(builtins)
    }
}
