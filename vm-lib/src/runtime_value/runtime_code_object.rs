// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use aria_compiler::line_table::LineTable;
use aria_parser::ast::SourcePointer;
use haxby_opcodes::Opcode;

#[derive(Clone)]
pub struct CodeObject {
    pub name: String,
    pub body: Rc<[Opcode]>,
    pub required_argc: u8,
    pub default_argc: u8,
    pub frame_size: u8,
    pub loc: SourcePointer,
    pub line_table: Rc<LineTable>,
}

impl PartialEq for CodeObject {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.body, &other.body)
    }
}

impl std::fmt::Debug for CodeObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<code-object {} at {}>", self.name, self.loc)
    }
}
