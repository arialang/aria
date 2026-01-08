// SPDX-License-Identifier: Apache-2.0

use std::cell::Cell;

use enum_as_inner::EnumAsInner;

use crate::shape::{ShapeId, SlotId};

#[derive(Clone, Copy)]
pub struct ReadAttributeSidecar {
    pub misses: u8,
    pub shape_id: ShapeId,
    pub slot_id: SlotId,
}

impl ReadAttributeSidecar {
    pub const MAXIMUM_ALLOWED_MISSES: u8 = 16;
}

#[derive(Clone, Copy, EnumAsInner)]
pub enum OpcodeSidecar {
    ReadAttribute(ReadAttributeSidecar),
}

pub type SidecarCell = Cell<Option<OpcodeSidecar>>;
pub type SidecarSlice = [SidecarCell];
