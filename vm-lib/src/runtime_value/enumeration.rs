// SPDX-License-Identifier: Apache-2.0

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use rustc_data_structures::fx::FxHashSet;

use crate::{
    builtins::VmGlobals,
    runtime_value::{
        function::{BuiltinFunctionImpl, Function},
        isa::IsaCheckable,
        object::ObjectBox,
    },
    shape::{ShapeId, SlotId},
    symbol::Symbol,
};

use super::{
    RuntimeValue,
    enum_case::{EnumValue, EnumValueImpl},
    mixin::Mixin,
};

#[derive(Clone)]
pub struct EnumCase {
    pub name: Symbol,
    pub payload_type: Option<IsaCheckable>,
}

pub struct EnumImpl {
    name: String,
    cases: RefCell<Vec<EnumCase>>,
    case_shape: Cell<ShapeId>,
    pub(super) entries: ObjectBox,
    mixins: RefCell<crate::mixin_includer::MixinIncluder>,
}

impl EnumImpl {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            cases: Default::default(),
            case_shape: Cell::new(crate::shape::Shapes::EMPTY_SHAPE_INDEX),
            entries: ObjectBox::default(),
            mixins: RefCell::new(crate::mixin_includer::MixinIncluder::default()),
        }
    }

    pub fn add_case(&self, builtins: &mut VmGlobals, case: EnumCase) -> usize {
        let (shape_id, slot_id) = builtins.shapes.transition(self.case_shape.get(), case.name);
        self.case_shape.set(shape_id);
        let slot_id = slot_id.0 as usize;
        let mut cases = self.cases.borrow_mut();
        if slot_id == cases.len() {
            cases.push(case);
        } else if slot_id < cases.len() {
            cases[slot_id] = case;
        } else {
            panic!("enum cases should grow sequentially");
        }
        slot_id
    }

    pub fn add_cases(&self, builtins: &mut VmGlobals, cases: &[EnumCase]) {
        for case in cases {
            self.add_case(builtins, case.clone());
        }
    }

    fn get_case_by_idx(&self, idx: usize) -> Option<EnumCase> {
        let b = self.cases.borrow();
        b.get(idx).cloned()
    }

    fn get_idx_of_case_by_symbol(&self, builtins: &VmGlobals, name: Symbol) -> Option<usize> {
        builtins
            .shapes
            .resolve_slot(self.case_shape.get(), name)
            .map(|slot_id| slot_id.0 as usize)
    }

    fn load_named_value(&self, builtins: &VmGlobals, name: Symbol) -> Option<RuntimeValue> {
        if let Some(nv) = self.entries.read(builtins, name) {
            Some(nv.clone())
        } else {
            self.mixins.borrow().load_named_value(builtins, name)
        }
    }

    fn store_named_value(&self, builtins: &mut VmGlobals, name: Symbol, val: RuntimeValue) {
        self.entries.write(builtins, name, val);
    }

    fn include_mixin(&self, mixin: &Mixin) {
        self.mixins.borrow_mut().include(mixin.clone());
    }

    fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.mixins.borrow().contains(mixin)
    }

    fn list_attributes(&self, builtins: &VmGlobals) -> FxHashSet<Symbol> {
        let mut attrs = self.entries.list_attributes(builtins);
        attrs.extend(self.mixins.borrow().list_attributes(builtins));
        attrs
    }

    fn case_shape_id(&self) -> ShapeId {
        self.case_shape.get()
    }

    pub(super) fn resolve_to_slot(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<(ShapeId, SlotId)> {
        let sid = self.case_shape_id();
        let slot_id = builtins.shapes.resolve_slot(sid, name)?;
        Some((sid, slot_id))
    }
}

impl Default for EnumImpl {
    fn default() -> Self {
        Self::new("")
    }
}

#[derive(Clone)]
pub struct Enum {
    pub(super) imp: Rc<EnumImpl>,
}

impl Enum {
    pub fn new(name: &str) -> Self {
        Self {
            imp: Rc::new(EnumImpl::new(name)),
        }
    }

    pub fn new_with_cases(name: &str, cases: &[EnumCase], builtins: &mut VmGlobals) -> Self {
        let enumm = Self::new(name);
        enumm.imp.add_cases(builtins, cases);
        enumm
    }

    pub fn name(&self) -> &str {
        &self.imp.name
    }

    pub fn add_case(&self, builtins: &mut VmGlobals, case: EnumCase) -> usize {
        self.imp.add_case(builtins, case)
    }

    pub fn get_idx_of_case_by_symbol(&self, builtins: &VmGlobals, name: Symbol) -> Option<usize> {
        self.imp.get_idx_of_case_by_symbol(builtins, name)
    }

    pub fn get_case_by_idx(&self, idx: usize) -> Option<EnumCase> {
        self.imp.get_case_by_idx(idx)
    }

    pub fn load_named_value(&self, builtins: &VmGlobals, name: Symbol) -> Option<RuntimeValue> {
        self.imp.load_named_value(builtins, name)
    }

    pub fn include_mixin(&self, mixin: &Mixin) {
        self.imp.include_mixin(mixin);
    }

    pub fn isa_mixin(&self, mixin: &Mixin) -> bool {
        self.imp.isa_mixin(mixin)
    }

    pub fn make_value(&self, cidx: usize, payload: Option<RuntimeValue>) -> Option<EnumValue> {
        match self.get_case_by_idx(cidx) {
            Some(case) => {
                if case.payload_type.is_some() == payload.is_some() {
                    Some(EnumValue {
                        imp: Rc::new(EnumValueImpl {
                            enumm: self.clone(),
                            case: cidx,
                            payload,
                        }),
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn list_attributes(&self, builtins: &VmGlobals) -> FxHashSet<Symbol> {
        self.imp.list_attributes(builtins)
    }

    pub(crate) fn case_shape_id(&self) -> ShapeId {
        self.imp.case_shape_id()
    }

    pub fn insert_builtin<T>(&self, builtins: &mut VmGlobals)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name = builtins
            .intern_symbol(t.name())
            .expect("too many symbols interned");
        self.imp.store_named_value(
            builtins,
            name,
            RuntimeValue::Function(Function::builtin_from(t)),
        );
    }

    pub fn resolve_to_slot(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<(ShapeId, SlotId)> {
        self.imp.resolve_to_slot(builtins, name)
    }
}

impl PartialEq for Enum {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for Enum {}
