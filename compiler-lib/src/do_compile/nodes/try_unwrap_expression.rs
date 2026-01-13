// SPDX-License-Identifier: Apache-2.0
use aria_parser::ast::TryUnwrapExpression;
use haxby_opcodes::BuiltinTypeId;

use crate::{
    builder::compiler_opcodes::CompilerOpcode,
    constant_value::ConstantValue,
    do_compile::{
        CompilationError, CompilationErrorReason, CompilationResult, CompileNode, CompileParams,
    },
};

impl<'a> CompileNode<'a> for TryUnwrapExpression {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        self.left.do_compile(params)?;

        let try_unwrap_protocol_idx = params
            .module
            .constants
            .insert(ConstantValue::String("try_unwrap_protocol".to_string()))
            .map_err(|_| CompilationError {
                loc: self.loc.clone(),
                reason: CompilationErrorReason::TooManyConstants,
            })?;

        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(
                CompilerOpcode::PushBuiltinTy(BuiltinTypeId::Result),
                self.loc.clone(),
            )
            .write_opcode_and_source_info(
                CompilerOpcode::ReadAttribute(try_unwrap_protocol_idx),
                self.loc.clone(),
            )
            .write_opcode_and_source_info(CompilerOpcode::Call(1), self.loc.clone())
            .write_opcode_and_source_info(
                CompilerOpcode::TryUnwrapProtocol(
                    haxby_opcodes::try_unwrap_protocol_mode::FLAG_TO_CALLER,
                ),
                self.loc.clone(),
            );

        let fallback_block = params
            .writer
            .append_block_at_end(&format!("try_unwrap_fallback_{}", self.loc));
        let end_block = params
            .writer
            .append_block_at_end(&format!("try_unwrap_end_{}", self.loc));

        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(
                CompilerOpcode::JumpConditionally(end_block.clone(), fallback_block.clone()),
                self.loc.clone(),
            );

        params.writer.set_current_block(fallback_block);
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(CompilerOpcode::Pop, self.loc.clone());
        self.right.do_compile(params)?;
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(
                CompilerOpcode::Jump(end_block.clone()),
                self.loc.clone(),
            );

        params.writer.set_current_block(end_block);

        Ok(())
    }
}
