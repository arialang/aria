// SPDX-License-Identifier: Apache-2.0
use aria_parser::ast::prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator};
use opcodes::opcode_prettyprint;

use crate::{
    bc_reader::BytecodeReader,
    constant_value::{CompiledCodeObject, ConstantValue, ConstantValues},
    module::CompiledModule,
};

pub mod opcodes;

pub trait StringResolver {
    fn resolve_compile_time_constant(&self, _: u16) -> Option<&str> {
        None
    }

    fn resolve_run_time_symbol(&self, _: u32) -> Option<&str> {
        None
    }
}

impl StringResolver for CompiledModule {
    fn resolve_compile_time_constant(&self, idx: u16) -> Option<&str> {
        match self.constants.values.get(idx as usize) {
            Some(ConstantValue::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }
}

trait ModuleDump {
    fn dump(
        &self,
        resolver: &dyn StringResolver,
        buffer: PrintoutAccumulator,
    ) -> PrintoutAccumulator;
}

impl ModuleDump for ConstantValue {
    fn dump(
        &self,
        resolver: &dyn StringResolver,
        buffer: PrintoutAccumulator,
    ) -> PrintoutAccumulator {
        match self {
            ConstantValue::Integer(n) => buffer << "int(" << n << ")",
            ConstantValue::String(s) => buffer << "str(\"" << s.as_str() << "\")",
            ConstantValue::Float(f) => buffer << "fp(" << f.raw_value() << ")",
            ConstantValue::CompiledCodeObject(cco) => cco.dump(resolver, buffer),
        }
    }
}

impl ModuleDump for ConstantValues {
    fn dump(
        &self,
        resolver: &dyn StringResolver,
        buffer: PrintoutAccumulator,
    ) -> PrintoutAccumulator {
        let mut dest = buffer;
        for cv in self.values.iter().enumerate() {
            dest = dest << "cv @" << cv.0 << " -> ";
            dest = cv.1.dump(resolver, dest) << "\n"
        }

        dest
    }
}

impl ModuleDump for CompiledCodeObject {
    fn dump(
        &self,
        resolver: &dyn StringResolver,
        buffer: PrintoutAccumulator,
    ) -> PrintoutAccumulator {
        let mut dest = buffer
            << "cco(name:\""
            << self.name.as_str()
            << " required arguments:"
            << self.required_argc
            << " default arguments:"
            << self.default_argc
            << " frame size:"
            << self.frame_size
            << ") bc=\n";

        let mut bcr = match BytecodeReader::try_from(self.body.as_slice()) {
            Ok(bcr) => bcr,
            Err(e) => {
                return dest << "    <invalid bytecode: " << e.to_string() << ">\n";
            }
        };
        let mut op_idx = 0;
        loop {
            let idx_str = format!("    {op_idx:05}: ");
            match bcr.read_opcode() {
                Ok(op) => {
                    dest = opcode_prettyprint(op, resolver, dest << idx_str);
                    if let Some(lte) = self.line_table.get(op_idx as u16) {
                        dest = dest << format!(" --> {lte}") << "\n";
                    } else {
                        dest = dest << "\n";
                    }
                    op_idx += 1;
                }
                Err(_) => {
                    break;
                }
            }
        }

        dest
    }
}

impl PrettyPrintable for CompiledModule {
    fn prettyprint(&self, buffer: PrintoutAccumulator) -> PrintoutAccumulator {
        self.constants.dump(self, buffer)
    }
}
