// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use rustc_data_structures::fx::{FxHashMap, FxHashSet};

use crate::error::vm_error::VmErrorReason;

use super::{RuntimeValue, structure::Struct};

#[derive(Default)]
pub struct ObjectBox {
    values: RefCell<FxHashMap<String, RuntimeValue>>,
}

impl ObjectBox {
    pub fn write(&self, name: &str, val: RuntimeValue) {
        self.values.borrow_mut().insert(name.to_owned(), val);
    }

    pub fn read(&self, name: &str) -> Option<RuntimeValue> {
        self.values.borrow().get(name).cloned()
    }

    fn delete(&self, name: &str) {
        self.values.borrow_mut().remove(name);
    }

    pub(super) fn list_attributes(&self) -> FxHashSet<String> {
        self.values.borrow().keys().cloned().collect()
    }

    pub(crate) fn contains(&self, name: &str) -> bool {
        self.values.borrow().contains_key(name)
    }

    pub(crate) fn keys(&self) -> FxHashSet<String> {
        self.values.borrow().keys().cloned().collect()
    }
}

pub(super) struct ObjectImpl {
    pub(super) boxx: ObjectBox,
    kind: Struct,
}

#[derive(Clone)]
pub struct Object {
    pub(super) imp: Rc<ObjectImpl>,
}

impl ObjectImpl {
    fn new(kind: &Struct) -> Self {
        Self {
            boxx: Default::default(),
            kind: kind.clone(),
        }
    }

    fn write(&self, name: &str, val: RuntimeValue) {
        self.boxx.write(name, val)
    }

    fn read(&self, name: &str) -> Option<RuntimeValue> {
        self.boxx.read(name)
    }

    fn delete(&self, name: &str) {
        self.boxx.delete(name);
    }

    fn list_attributes(&self) -> FxHashSet<String> {
        self.boxx.list_attributes()
    }
}

impl Object {
    pub fn new(kind: &Struct) -> Self {
        Self {
            imp: Rc::new(ObjectImpl::new(kind)),
        }
    }

    pub fn read(&self, name: &str) -> Option<RuntimeValue> {
        self.imp.read(name)
    }

    pub fn list_attributes(&self) -> FxHashSet<String> {
        self.imp.list_attributes()
    }

    pub fn delete(&self, name: &str) {
        self.imp.delete(name);
    }

    pub fn get_struct(&self) -> &Struct {
        &self.imp.kind
    }

    pub fn with_value(self, name: &str, val: RuntimeValue) -> Self {
        self.imp.write(name, val);
        self
    }
}

impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for Object {}

impl Object {
    pub fn extract_field<FnType, OkType>(
        &self,
        name: &str,
        f: FnType,
    ) -> Result<OkType, VmErrorReason>
    where
        FnType: FnOnce(RuntimeValue) -> Option<OkType>,
    {
        let val = match self.read(name) {
            Some(v) => v,
            None => {
                return Err(VmErrorReason::NoSuchIdentifier(name.to_owned()));
            }
        };

        match f(val) {
            Some(v) => Ok(v),
            None => Err(VmErrorReason::UnexpectedType),
        }
    }
}
