// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use aria_compiler::{
    bc_reader::{BytecodeReader, DecodeResult},
    constant_value::CompiledCodeObject,
    line_table::LineTable,
};
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

fn byte_array_to_opcode_array(bytes: &[u8]) -> DecodeResult<Vec<Opcode>> {
    let mut opcodes = Vec::new();
    let mut decoder = BytecodeReader::from(bytes);

    loop {
        let next = decoder.read_opcode();
        match next {
            Ok(op) => opcodes.push(op),
            Err(err) => {
                return match err {
                    aria_compiler::bc_reader::DecodeError::EndOfStream => Ok(opcodes),
                    _ => Err(err),
                };
            }
        }
    }
}

impl TryFrom<&CompiledCodeObject> for CodeObject {
    type Error = aria_compiler::bc_reader::DecodeError;

    fn try_from(value: &CompiledCodeObject) -> Result<Self, Self::Error> {
        let ops = byte_array_to_opcode_array(value.body.as_slice())?;
        let body: Rc<[Opcode]> = ops.into();

        Ok(Self {
            name: value.name.clone(),
            body,
            required_argc: value.required_argc,
            default_argc: value.default_argc,
            frame_size: value.frame_size,
            loc: value.loc.clone(),
            line_table: Rc::from(value.line_table.clone()),
        })
    }
}

impl std::fmt::Debug for CodeObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<code-object {} at {}>", self.name, self.loc)
    }
}
