// SPDX-License-Identifier: Apache-2.0
use aria_parser::ast::prettyprint::printout_accumulator::PrintoutAccumulator;
use haxby_opcodes::Opcode;

use crate::runtime_module::RuntimeModule;

pub(crate) fn opcode_prettyprint(
    opcode: Opcode,
    module: &RuntimeModule,
    buffer: PrintoutAccumulator,
) -> PrintoutAccumulator {
    aria_compiler::dump::opcodes::opcode_prettyprint(opcode, module.get_compiled_module(), buffer)
}
