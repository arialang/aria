// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use rustc_data_structures::fx::FxHashSet;

use crate::{
    builtins::VmGlobals, error::vm_error::VmErrorReason, runtime_value::object::ObjectBox,
    symbol::Symbol,
};

use super::{
    RuntimeValue,
    function::{BuiltinFunctionImpl, Function},
    mixin::Mixin,
};

struct StructImpl {
    name: String,
    entries: ObjectBox,
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

    fn load_named_value(&self, name: Symbol) -> Option<RuntimeValue> {
        if let Some(nv) = self.entries.read(name) {
            Some(nv.clone())
        } else {
            self.mixins.borrow().load_named_value(name)
        }
    }

    fn store_named_value(&self, name: Symbol, val: RuntimeValue) {
        self.entries.write(name, val);
    }

    fn include_mixin(&self, mixin: &Mixin) {
        self.mixins.borrow_mut().include(mixin.clone());
    }

    fn list_attributes(&self) -> FxHashSet<Symbol> {
        let mut attrs = self.entries.keys();
        attrs.extend(self.mixins.borrow().list_attributes());
        attrs
    }
}

#[derive(Clone)]
pub struct Struct {
    imp: Rc<StructImpl>,
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

    pub fn load_named_value(&self, name: Symbol) -> Option<RuntimeValue> {
        self.imp.load_named_value(name)
    }

    pub fn store_named_value(&self, name: Symbol, val: RuntimeValue) {
        self.imp.store_named_value(name, val);
    }

    pub fn include_mixin(&self, mixin: &Mixin) {
        self.imp.include_mixin(mixin);
    }

    pub fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.imp.isa_mixin(mixin)
    }

    pub fn list_attributes(&self) -> FxHashSet<Symbol> {
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
    pub fn insert_builtin<T>(&self, builtins: &VmGlobals)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name_sym = builtins
            .intern_symbol(t.name())
            .expect("failed to intern builtin name");
        self.store_named_value(name_sym, RuntimeValue::Function(Function::builtin_from(t)));
    }

    pub fn extract_field<FnType, OkType>(
        &self,
        name: Symbol,
        builtins: &VmGlobals,
        f: FnType,
    ) -> Result<OkType, VmErrorReason>
    where
        FnType: FnOnce(RuntimeValue) -> Option<OkType>,
    {
        let val = match self.load_named_value(name) {
            Some(v) => v,
            None => {
                return Err(match builtins.resolve_symbol(name) {
                    Some(sym_name) => VmErrorReason::NoSuchIdentifier(sym_name),
                    None => VmErrorReason::NoSuchSymbol(name),
                });
            }
        };

        match f(val) {
            Some(v) => Ok(v),
            None => Err(VmErrorReason::UnexpectedType),
        }
    }
}
