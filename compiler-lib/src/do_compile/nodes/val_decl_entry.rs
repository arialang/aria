// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::BuiltinTypeId;

use crate::{
    builder::compiler_opcodes::CompilerOpcode,
    do_compile::{
        CompilationError, CompilationErrorReason, CompilationResult, CompileNode, CompileParams,
    },
};

impl<'a> CompileNode<'a> for aria_parser::ast::ValDeclEntry {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        match self.id.name.value.as_str() {
            "true" | "false" => {
                return Err(CompilationError {
                    loc: self.loc.clone(),
                    reason: CompilationErrorReason::ReservedIdentifier(self.id.name.value.clone()),
                });
            }
            _ => {}
        };

        self.val.do_compile(params)?;
        if let Some(ty) = &self.id.ty {
            ty.do_compile(params)?;
        } else {
            params
                .writer
                .get_current_block()
                .write_opcode_and_source_info(
                    CompilerOpcode::PushBuiltinTy(BuiltinTypeId::Any),
                    self.loc.clone(),
                );
        }
        params.scope.emit_typed_define(
            &self.id.name.value,
            &mut params.module.constants,
            params.writer.get_current_block(),
            self.loc.clone(),
        )?;
        params.scope.emit_write(
            &self.id.name.value,
            &mut params.module.constants,
            params.writer.get_current_block(),
            self.loc.clone(),
        )?;
        Ok(())
    }
}
