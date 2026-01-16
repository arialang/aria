// SPDX-License-Identifier: Apache-2.0
use std::{
    cell::{Cell, UnsafeCell},
    rc::Rc,
};

use rustc_data_structures::fx::FxHashSet;

use crate::{error::vm_error::VmErrorReason, shape::ShapeId};
use crate::{shape::SlotId, symbol::Symbol};

use super::{RuntimeValue, structure::Struct};

pub struct ObjectBox {
    shape: Cell<ShapeId>,
    slots: UnsafeCell<Vec<RuntimeValue>>,
}

impl Default for ObjectBox {
    fn default() -> Self {
        Self {
            shape: Cell::new(crate::shape::Shapes::EMPTY_SHAPE_INDEX),
            slots: UnsafeCell::new(Vec::new()),
        }
    }
}

impl ObjectBox {
    #[allow(clippy::mut_from_ref)]
    #[inline]
    fn get(&self) -> &Vec<RuntimeValue> {
        unsafe { &*self.slots.get() }
    }

    #[allow(clippy::mut_from_ref)]
    #[inline]
    fn get_mut(&self) -> &mut Vec<RuntimeValue> {
        unsafe { &mut *self.slots.get() }
    }

    pub fn write(
        &self,
        builtins: &mut crate::builtins::VmGlobals,
        name: Symbol,
        val: RuntimeValue,
    ) {
        let (shape_id, slot_id) = builtins.shapes.transition(self.shape.get(), name);
        self.shape.set(shape_id);
        let slot_id = slot_id.0 as usize;
        let slot_count = self.get().len();
        if slot_id == slot_count {
            self.get_mut().push(val);
        } else if slot_id < slot_count {
            self.get_mut()[slot_id] = val;
        } else {
            panic!("slots should grow sequentially");
        }
    }

    pub fn read(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<RuntimeValue> {
        let slot_id = builtins.shapes.resolve_slot(self.shape.get(), name)?;
        self.get().get(slot_id.0 as usize).cloned()
    }

    pub(super) fn read_slot(&self, slot_id: SlotId, sid: ShapeId) -> Option<RuntimeValue> {
        if self.shape.get() != sid {
            return None;
        }
        self.get().get(slot_id.0 as usize).cloned()
    }

    pub(super) fn resolve_to_slot(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<(RuntimeValue, ShapeId, SlotId)> {
        let sid = self.shape.get();
        let slot_id = builtins.shapes.resolve_slot(sid, name)?;
        let val = self.get().get(slot_id.0 as usize)?.clone();
        Some((val, sid, slot_id))
    }

    pub(super) fn list_attributes(
        &self,
        builtins: &crate::builtins::VmGlobals,
    ) -> FxHashSet<Symbol> {
        let mut ret = FxHashSet::<Symbol>::default();
        let shape = match builtins.shapes.get_shape(self.shape.get()) {
            Some(s) => s,
            None => return ret,
        };

        assert_eq!(self.get().len(), shape.reverse_slots.len());

        shape.reverse_slots.iter().for_each(|&sym| {
            ret.insert(sym);
        });
        ret
    }

    pub(crate) fn contains(&self, builtins: &crate::builtins::VmGlobals, name: Symbol) -> bool {
        let slot_count = self.get().len();
        if let Some(slot_id) = builtins.shapes.resolve_slot(self.shape.get(), name) {
            (slot_id.0 as usize) < slot_count
        } else {
            false
        }
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

    fn read_slot(&self, slot_id: SlotId, sid: ShapeId) -> Option<RuntimeValue> {
        self.boxx.read_slot(slot_id, sid)
    }

    fn resolve_to_slot(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<(RuntimeValue, ShapeId, SlotId)> {
        self.boxx.resolve_to_slot(builtins, name)
    }

    fn write(&self, builtins: &mut crate::builtins::VmGlobals, name: Symbol, val: RuntimeValue) {
        self.boxx.write(builtins, name, val)
    }

    fn read(&self, builtins: &crate::builtins::VmGlobals, name: Symbol) -> Option<RuntimeValue> {
        self.boxx.read(builtins, name)
    }

    fn list_attributes(&self, builtins: &crate::builtins::VmGlobals) -> FxHashSet<Symbol> {
        self.boxx.list_attributes(builtins)
    }
}

impl Object {
    pub fn new(kind: &Struct) -> Self {
        Self {
            imp: Rc::new(ObjectImpl::new(kind)),
        }
    }

    pub(crate) fn read_slot(&self, slot_id: SlotId, sid: ShapeId) -> Option<RuntimeValue> {
        self.imp.read_slot(slot_id, sid)
    }

    pub(crate) fn resolve_to_slot(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<(RuntimeValue, ShapeId, SlotId)> {
        self.imp.resolve_to_slot(builtins, name)
    }

    pub fn read(
        &self,
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
    ) -> Option<RuntimeValue> {
        self.imp.read(builtins, name)
    }

    pub fn write(
        &self,
        builtins: &mut crate::builtins::VmGlobals,
        name: Symbol,
        val: RuntimeValue,
    ) {
        self.imp.write(builtins, name, val)
    }

    pub fn list_attributes(&self, builtins: &crate::builtins::VmGlobals) -> FxHashSet<Symbol> {
        self.imp.list_attributes(builtins)
    }

    pub fn get_struct(&self) -> &Struct {
        &self.imp.kind
    }

    pub fn with_value(
        self,
        builtins: &mut crate::builtins::VmGlobals,
        name: Symbol,
        val: RuntimeValue,
    ) -> Self {
        self.imp.write(builtins, name, val);
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
        builtins: &crate::builtins::VmGlobals,
        name: Symbol,
        f: FnType,
    ) -> Result<OkType, VmErrorReason>
    where
        FnType: FnOnce(RuntimeValue) -> Option<OkType>,
    {
        let val = match self.read(builtins, name) {
            Some(v) => v,
            None => {
                return Err(VmErrorReason::NoSuchIdentifier(name.to_string()));
            }
        };

        match f(val) {
            Some(v) => Ok(v),
            None => Err(VmErrorReason::UnexpectedType),
        }
    }
}
