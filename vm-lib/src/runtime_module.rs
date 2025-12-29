// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use aria_compiler::{constant_value::ConstantValue, module::CompiledModule};
use rustc_data_structures::fx::FxHashMap;

use crate::{
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
    runtime_value::{
        RuntimeValue,
        function::{BuiltinFunctionImpl, Function},
        isa::IsaCheckable,
    },
};

#[derive(Clone)]
pub struct NamedValue {
    pub val: RuntimeValue,
    pub ty: IsaCheckable,
}

struct RuntimeModuleImpl {
    compiled_module: CompiledModule,
    values: RefCell<FxHashMap<String, NamedValue>>,
}

impl RuntimeModuleImpl {
    fn new(cm: CompiledModule) -> Self {
        Self {
            compiled_module: cm,
            values: Default::default(),
        }
    }

    fn named_values_of_this(&self) -> Vec<(String, NamedValue)> {
        let mut ret = vec![];

        for (n, v) in self.values.borrow().iter() {
            ret.push((n.clone(), v.clone()));
        }

        ret
    }

    fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        self.values.borrow().get(name).map(|v| v.val.clone())
    }

    fn typedef_named_value(&self, name: &str, ty: IsaCheckable) {
        let mut bm = self.values.borrow_mut();
        if let Some(val) = bm.get_mut(name) {
            val.ty = ty;
        } else {
            bm.insert(
                name.to_owned(),
                NamedValue {
                    val: RuntimeValue::Integer(0.into()),
                    ty,
                },
            );
        }
    }

    fn store_typechecked_named_value(
        &self,
        name: &str,
        val: RuntimeValue,
        builtins: &VmGlobals,
    ) -> Result<(), VmErrorReason> {
        let mut bm = self.values.borrow_mut();
        if let Some(nval) = bm.get_mut(name) {
            if !nval.ty.isa_check(&val, builtins) {
                Err(VmErrorReason::UnexpectedType)
            } else {
                nval.val = val;
                Ok(())
            }
        } else {
            Err(VmErrorReason::NoSuchIdentifier(name.to_owned()))
        }
    }

    fn store_named_value(&self, name: &str, val: RuntimeValue) {
        let mut bm = self.values.borrow_mut();
        if let Some(nval) = bm.get_mut(name) {
            nval.val = val;
        } else {
            bm.insert(
                name.to_owned(),
                NamedValue {
                    val,
                    ty: IsaCheckable::any(),
                },
            );
        }
    }

    fn load_indexed_const(&self, idx: u16) -> Option<ConstantValue> {
        self.compiled_module.load_indexed_const(idx)
    }

    fn list_named_values(&self) -> HashSet<String> {
        self.values.borrow().keys().cloned().collect()
    }
}

#[derive(Clone)]
pub struct RuntimeModule {
    imp: Rc<RuntimeModuleImpl>,
}

impl RuntimeModule {
    pub fn new(cm: CompiledModule) -> Self {
        Self {
            imp: Rc::new(RuntimeModuleImpl::new(cm)),
        }
    }

    pub(crate) fn named_values_of_this(&self) -> Vec<(String, NamedValue)> {
        self.imp.named_values_of_this()
    }

    pub(crate) fn get_compiled_module(&self) -> &CompiledModule {
        &self.imp.compiled_module
    }

    pub fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        self.imp.load_named_value(name)
    }

    pub fn typedef_named_value(&self, name: &str, ty: IsaCheckable) {
        self.imp.typedef_named_value(name, ty)
    }

    pub fn store_named_value(&self, name: &str, val: RuntimeValue) {
        self.imp.store_named_value(name, val)
    }

    pub fn list_named_values(&self) -> HashSet<String> {
        self.imp.list_named_values()
    }

    pub fn store_typechecked_named_value(
        &self,
        name: &str,
        val: RuntimeValue,
        builtins: &VmGlobals,
    ) -> Result<(), VmErrorReason> {
        self.imp.store_typechecked_named_value(name, val, builtins)
    }

    pub fn load_indexed_const(&self, idx: u16) -> Option<ConstantValue> {
        self.imp.load_indexed_const(idx)
    }

    pub fn lift_all_symbols_from_other(
        &self,
        prior_art: &Self,
        vm: &crate::VirtualMachine,
    ) -> Result<(), VmErrorReason> {
        for (name, val) in prior_art.named_values_of_this() {
            self.typedef_named_value(&name, val.ty.clone());
            self.store_typechecked_named_value(&name, val.val.clone(), &vm.globals)?;
        }
        Ok(())
    }

    pub fn extract_value<T, U>(&self, name: &str, f: T) -> Option<U>
    where
        T: FnOnce(RuntimeValue) -> Option<U>,
    {
        f(self.load_named_value(name)?)
    }

    pub fn insert_builtin<T>(&self)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name = t.name().to_owned();
        self.store_named_value(&name, RuntimeValue::Function(Function::builtin_from(t)));
    }
}

impl PartialEq for RuntimeModule {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for RuntimeModule {}
