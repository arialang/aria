// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, rc::Rc};

use enum_as_inner::EnumAsInner;
use rustc_data_structures::fx::FxHashSet;

use super::{
    RuntimeValue,
    function::{BuiltinFunctionImpl, Function},
    mixin::Mixin,
    object::ObjectBox,
};

#[derive(EnumAsInner, Clone, PartialEq, Eq)]
pub enum RustNativeValueKind {
    Boolean,
    Integer,
    Float,
    List,
    String,
    Type,
}

struct RustNativeTypeImpl {
    tag: RustNativeValueKind,
    boxx: Rc<ObjectBox>,
    mixins: RefCell<crate::mixin_includer::MixinIncluder>,
}

impl RustNativeTypeImpl {
    fn write(&self, name: &str, val: RuntimeValue) {
        self.boxx.write(name, val)
    }

    fn read(&self, name: &str) -> Option<RuntimeValue> {
        match self.boxx.read(name) {
            Some(nv) => Some(nv),
            _ => self.mixins.borrow().load_named_value(name),
        }
    }

    fn include_mixin(&self, mixin: &Mixin) {
        self.mixins.borrow_mut().include(mixin.clone());
    }

    fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.mixins.borrow().contains(mixin)
    }

    fn list_attributes(&self) -> FxHashSet<String> {
        let mut attrs = self.boxx.list_attributes();
        attrs.extend(self.mixins.borrow().list_attributes());
        attrs
    }
}

#[derive(Clone)]
pub struct RustNativeType {
    imp: Rc<RustNativeTypeImpl>,
}

impl RustNativeType {
    pub fn new(rvt: RustNativeValueKind) -> Self {
        Self {
            imp: Rc::new(RustNativeTypeImpl {
                tag: rvt,
                boxx: Rc::new(Default::default()),
                mixins: Default::default(),
            }),
        }
    }

    pub fn get_tag(&self) -> &RustNativeValueKind {
        &self.imp.tag
    }

    pub fn get_boxx(&self) -> &Rc<ObjectBox> {
        &self.imp.boxx
    }

    pub(crate) fn write(&self, name: &str, val: RuntimeValue) {
        self.imp.write(name, val);
    }

    pub fn read(&self, name: &str) -> Option<RuntimeValue> {
        self.imp.read(name)
    }

    pub fn include_mixin(&self, mixin: &Mixin) {
        self.imp.include_mixin(mixin);
    }

    pub fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.imp.isa_mixin(mixin)
    }

    pub fn insert_builtin<T>(&self)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name = t.name().to_owned();
        self.get_boxx()
            .write(&name, RuntimeValue::Function(Function::builtin_from(t)));
    }

    pub fn list_attributes(&self) -> FxHashSet<String> {
        self.imp.list_attributes()
    }
}

impl PartialEq for RustNativeType {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp) || self.imp.tag == other.imp.tag
    }
}
impl Eq for RustNativeType {}

impl std::fmt::Debug for RustNativeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.get_tag() {
            RustNativeValueKind::Boolean => write!(f, "Bool"),
            RustNativeValueKind::Integer => write!(f, "Int"),
            RustNativeValueKind::Float => write!(f, "Float"),
            RustNativeValueKind::List => write!(f, "List"),
            RustNativeValueKind::String => write!(f, "String"),
            RustNativeValueKind::Type => write!(f, "Type"),
        }
    }
}
