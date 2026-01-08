// SPDX-License-Identifier: Apache-2.0
use rustc_data_structures::fx::FxHashMap;

use crate::symbol::Symbol;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ShapeId(pub u32);

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct SlotId(pub u32);

pub struct Shape {
    #[allow(unused)]
    pub(crate) id: ShapeId,
    pub(crate) slots: FxHashMap<Symbol, SlotId>,
    pub(crate) reverse_slots: Vec<Symbol>,
    pub(crate) transitions: FxHashMap<Symbol, ShapeId>,
}

impl Shape {
    pub fn empty() -> Self {
        Shape {
            id: Shapes::EMPTY_SHAPE_INDEX,
            slots: Default::default(),
            reverse_slots: Vec::default(),
            transitions: Default::default(),
        }
    }
}

pub struct Shapes {
    shapes: Vec<Shape>,
}

impl Default for Shapes {
    fn default() -> Self {
        Self {
            shapes: vec![Shape::empty()],
        }
    }
}

impl Shapes {
    pub const EMPTY_SHAPE_INDEX: ShapeId = ShapeId(0);

    pub fn transition(&mut self, cur_sid: ShapeId, name: Symbol) -> (ShapeId, SlotId) {
        let cur_shape = &self.shapes[cur_sid.0 as usize];
        if let Some(cur_slot) = cur_shape.slots.get(&name) {
            return (cur_sid, *cur_slot);
        }

        if let Some(next_sid) = cur_shape.transitions.get(&name) {
            let slot_id = self.shapes[next_sid.0 as usize]
                .slots
                .get(&name)
                .expect("transition shape missing slot");
            return (*next_sid, *slot_id);
        }

        let new_slot_id = SlotId(cur_shape.slots.len() as u32);
        let mut new_shape_slots = cur_shape.slots.clone();
        new_shape_slots.insert(name, new_slot_id);
        let mut new_shape_reverse_slots = cur_shape.reverse_slots.clone();
        assert_eq!(new_shape_reverse_slots.len(), new_slot_id.0 as usize);
        new_shape_reverse_slots.push(name);

        let new_sid = ShapeId(self.shapes.len() as u32);
        let new_shape = Shape {
            id: new_sid,
            slots: new_shape_slots,
            reverse_slots: new_shape_reverse_slots,
            transitions: FxHashMap::default(),
        };
        self.shapes.push(new_shape);

        let cur_shape = &mut self.shapes[cur_sid.0 as usize];
        cur_shape.transitions.insert(name, new_sid);

        (new_sid, new_slot_id)
    }

    pub fn resolve_slot(&self, sid: ShapeId, name: Symbol) -> Option<SlotId> {
        self.shapes
            .get(sid.0 as usize)
            .and_then(|shape| shape.slots.get(&name).copied())
    }

    pub fn resolve_symbol(&self, sid: ShapeId, slot_id: SlotId) -> Option<Symbol> {
        self.shapes
            .get(sid.0 as usize)
            .and_then(|shape| shape.reverse_slots.get(slot_id.0 as usize).copied())
    }

    pub(crate) fn get_shape(&self, sid: ShapeId) -> Option<&Shape> {
        self.shapes.get(sid.0 as usize)
    }
}
