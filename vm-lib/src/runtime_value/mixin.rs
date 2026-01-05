// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use rustc_data_structures::fx::FxHashSet;

use crate::runtime_value::object::ObjectBox;
use crate::symbol::Symbol;

use super::RuntimeValue;

struct MixinImpl {
    name: String,
    entries: ObjectBox,
    mixins: RefCell<crate::mixin_includer::MixinIncluder>,
}

impl MixinImpl {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            entries: ObjectBox::default(),
            mixins: RefCell::new(crate::mixin_includer::MixinIncluder::default()),
        }
    }

    fn load_named_value(&self, name: Symbol) -> Option<RuntimeValue> {
        if let Some(val) = self.entries.read(name) {
            Some(val.clone())
        } else {
            self.mixins.borrow().load_named_value(name)
        }
    }

    fn store_named_value(&self, name: Symbol, val: RuntimeValue) {
        self.entries.write(name, val);
    }

    fn named_values(&self) -> Vec<Symbol> {
        self.entries.keys().into_iter().collect()
    }

    fn include_mixin(&self, mixin: &Mixin) {
        self.mixins.borrow_mut().include(mixin.clone());
    }

    fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.mixins.borrow().contains(mixin)
    }

    fn list_attributes(&self) -> FxHashSet<Symbol> {
        let mut attrs = self.entries.keys();
        attrs.extend(self.mixins.borrow().list_attributes());
        attrs
    }
}

#[derive(Clone)]
pub struct Mixin {
    imp: Rc<MixinImpl>,
}

impl Mixin {
    pub fn new(name: &str) -> Self {
        Self {
            imp: Rc::new(MixinImpl::new(name)),
        }
    }

    pub fn name(&self) -> &str {
        &self.imp.name
    }

    pub fn load_named_value(&self, name: Symbol) -> Option<RuntimeValue> {
        self.imp.load_named_value(name)
    }

    pub fn store_named_value(&self, name: Symbol, val: RuntimeValue) {
        self.imp.store_named_value(name, val);
    }

    pub fn named_values(&self) -> Vec<Symbol> {
        self.imp.named_values()
    }

    pub fn include_mixin(&self, mixin: &Mixin) {
        self.imp.include_mixin(mixin);
    }

    pub fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self == mixin || self.imp.isa_mixin(mixin)
    }

    pub fn list_attributes(&self) -> FxHashSet<Symbol> {
        self.imp.list_attributes()
    }
}

impl PartialEq for Mixin {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for Mixin {}
