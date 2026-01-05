// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use rustc_data_structures::fx::FxHashSet;

use super::{RuntimeValue, object::ObjectBox};
use crate::symbol::Symbol;

pub(crate) struct BuiltinValueImpl<T>
where
    T: Clone,
{
    pub(crate) val: T,
    pub(crate) boxx: ObjectBox,
}

impl<T> BuiltinValueImpl<T>
where
    T: Clone,
{
    fn write(&self, name: Symbol, val: RuntimeValue) {
        self.boxx.write(name, val)
    }

    fn read(&self, name: Symbol) -> Option<RuntimeValue> {
        self.boxx.read(name)
    }

    fn list_attributes(&self) -> FxHashSet<Symbol> {
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

impl<T> From<T> for BuiltinValueImpl<T>
where
    T: Clone,
{
    fn from(val: T) -> Self {
        Self {
            val,
            boxx: Default::default(),
        }
    }
}

impl<T> From<T> for BuiltinValue<T>
where
    T: Clone,
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
    pub fn raw_value(&self) -> T {
        self.imp.val.clone()
    }

    pub fn write(&self, name: Symbol, val: RuntimeValue) {
        self.imp.write(name, val)
    }

    pub fn read(&self, name: Symbol) -> Option<RuntimeValue> {
        self.imp.read(name)
    }

    pub fn list_attributes(&self) -> FxHashSet<Symbol> {
        self.imp.list_attributes()
    }
}
