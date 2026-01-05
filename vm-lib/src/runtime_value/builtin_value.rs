// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use haxby_opcodes::BuiltinTypeId;
use rustc_data_structures::fx::FxHashSet;

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
    fn list_attributes(&self) -> FxHashSet<String> {
        self.boxx.list_attributes()
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
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::Int
    }
}
impl GetBuiltinTypeId for bool {
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::Bool
    }
}
impl GetBuiltinTypeId for String {
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::String
    }
}
impl GetBuiltinTypeId for f64 {
    fn get_builtin_type_id() -> BuiltinTypeId {
        BuiltinTypeId::Float
    }
}

impl<T> From<T> for BuiltinValueImpl<T>
where
    T: Clone + GetBuiltinTypeId,
{
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
    pub fn builtin_type_id(&self) -> BuiltinTypeId {
        self.imp.id
    }

    pub fn raw_value(&self) -> T {
        self.imp.val.clone()
    }

    pub fn list_attributes(&self) -> FxHashSet<String> {
        self.imp.list_attributes()
    }
}
