// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use rustc_data_structures::fx::FxHashSet;

use crate::{error::vm_error::VmErrorReason, runtime_value::object::ObjectBox};

use super::{
    RuntimeValue,
    function::{BuiltinFunctionImpl, Function},
    mixin::Mixin,
};

pub(super) struct StructImpl {
    name: String,
    pub(super) entries: ObjectBox,
    mixins: RefCell<crate::mixin_includer::MixinIncluder>,
}

impl StructImpl {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            entries: ObjectBox::default(),
            mixins: RefCell::new(crate::mixin_includer::MixinIncluder::default()),
        }
    }

    fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.mixins.borrow().contains(mixin)
    }

    fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        if let Some(nv) = self.entries.read(name) {
            Some(nv.clone())
        } else {
            self.mixins.borrow().load_named_value(name)
        }
    }

    fn store_named_value(&self, name: &str, val: RuntimeValue) {
        self.entries.write(name, val);
    }

    fn include_mixin(&self, mixin: &Mixin) {
        self.mixins.borrow_mut().include(mixin.clone());
    }

    fn list_attributes(&self) -> FxHashSet<String> {
        let mut attrs = self.entries.keys();
        attrs.extend(self.mixins.borrow().list_attributes());
        attrs
    }
}

#[derive(Clone)]
pub struct Struct {
    pub(super) imp: Rc<StructImpl>,
}

impl Struct {
    pub fn new(name: &str) -> Self {
        Self {
            imp: Rc::new(StructImpl::new(name)),
        }
    }

    pub fn name(&self) -> &str {
        &self.imp.name
    }

    pub fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        self.imp.load_named_value(name)
    }

    pub(crate) fn store_named_value(&self, name: &str, val: RuntimeValue) {
        self.imp.store_named_value(name, val);
    }

    pub fn include_mixin(&self, mixin: &Mixin) {
        self.imp.include_mixin(mixin);
    }

    pub fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.imp.isa_mixin(mixin)
    }

    pub fn list_attributes(&self) -> FxHashSet<String> {
        self.imp.list_attributes()
    }
}

impl PartialEq for Struct {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for Struct {}

impl Struct {
    pub fn insert_builtin<T>(&self)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name = t.name().to_owned();
        self.store_named_value(&name, RuntimeValue::Function(Function::builtin_from(t)));
    }

    pub fn extract_field<FnType, OkType>(
        &self,
        name: &str,
        f: FnType,
    ) -> Result<OkType, VmErrorReason>
    where
        FnType: FnOnce(RuntimeValue) -> Option<OkType>,
    {
        let val = match self.load_named_value(name) {
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
