// SPDX-License-Identifier: Apache-2.0

use std::cell::Cell;

use aria_parser::ast::prettyprint::printout_accumulator::PrintoutAccumulator;
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

#[derive(Clone, Copy)]
pub struct NewEnumValSidecar {
    pub misses: u8,
    pub shape_id: ShapeId,
    pub slot_id: SlotId,
}

impl NewEnumValSidecar {
    pub const MAXIMUM_ALLOWED_MISSES: u8 = 16;
}

#[derive(Clone, Copy, EnumAsInner)]
pub enum OpcodeSidecar {
    ReadAttribute(ReadAttributeSidecar),
    NewEnumVal(NewEnumValSidecar),
}

pub type SidecarCell = Cell<Option<OpcodeSidecar>>;
pub type SidecarSlice = [SidecarCell];

pub(crate) fn sidecar_prettyprint(
    sidecar: OpcodeSidecar,
    buffer: PrintoutAccumulator,
) -> PrintoutAccumulator {
    match sidecar {
        OpcodeSidecar::ReadAttribute(sc) => {
            buffer
                << "[sidecar misses="
                << sc.misses
                << " shape_id="
                << sc.shape_id.0
                << " slot_id="
                << sc.slot_id.0
                << "]"
        }
        OpcodeSidecar::NewEnumVal(sc) => {
            buffer
                << "[enum_case misses="
                << sc.misses
                << " shape_id="
                << sc.shape_id.0
                << " slot_id="
                << sc.slot_id.0
                << "]"
        }
    }
}
