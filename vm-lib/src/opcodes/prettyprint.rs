// SPDX-License-Identifier: Apache-2.0

use aria_compiler::dump::StringResolver;
use aria_parser::ast::prettyprint::printout_accumulator::PrintoutAccumulator;
use haxby_opcodes::Opcode;

use crate::{builtins::VmGlobals, runtime_module::RuntimeModule};

pub(crate) struct RuntimeOpcodePrinter<'a> {
    pub(crate) globals: Option<&'a VmGlobals>,
    pub(crate) module: Option<&'a RuntimeModule>,
}

impl<'a> StringResolver for RuntimeOpcodePrinter<'a> {
    fn resolve_compile_time_constant(&self, idx: u16) -> Option<&str> {
        if let Some(module) = &self.module {
            module
                .get_compiled_module()
                .resolve_compile_time_constant(idx)
        } else {
            None
        }
    }

    fn resolve_run_time_symbol(&self, idx: u32) -> Option<&str> {
        if let Some(globals) = &self.globals {
            globals.resolve_symbol(crate::symbol::Symbol(idx))
        } else {
            None
        }
    }
}

pub(crate) fn opcode_prettyprint(
    opcode: Opcode,
    ropc: &RuntimeOpcodePrinter,
    buffer: PrintoutAccumulator,
) -> PrintoutAccumulator {
    aria_compiler::dump::opcodes::opcode_prettyprint(opcode, ropc, buffer)
}
