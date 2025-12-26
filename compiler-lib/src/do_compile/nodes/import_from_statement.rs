// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::BuiltinValueId;

use crate::{
    builder::compiler_opcodes::CompilerOpcode,
    constant_value::ConstantValue,
    do_compile::{CompilationResult, CompileNode, CompileParams},
};

impl<'a> CompileNode<'a> for aria_parser::ast::ImportFromStatement {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        let path_idx = self.insert_const_or_fail(
            params,
            ConstantValue::String(self.from.to_dotted_string()),
            &self.loc,
        )?;

        match &self.what {
            aria_parser::ast::ImportTarget::IdentifierList(identifiers) => {
                for identifier in &identifiers.identifiers {
                    let ident_idx = self.insert_const_or_fail(
                        params,
                        ConstantValue::String(identifier.value.clone()),
                        &self.loc,
                    )?;
                    params
                        .writer
                        .get_current_block()
                        .write_opcode_and_source_info(
                            CompilerOpcode::Import(path_idx),
                            self.loc.clone(),
                        )
                        .write_opcode_and_source_info(
                            CompilerOpcode::ReadAttribute(ident_idx),
                            self.loc.clone(),
                        );
                    params.scope.emit_untyped_define(
                        &identifier.value,
                        &mut params.module.constants,
                        params.writer.get_current_block(),
                        self.loc.clone(),
                    )?;
                }
            }
            aria_parser::ast::ImportTarget::All => {
                let path_idx = self.insert_const_or_fail(
                    params,
                    ConstantValue::String(self.from.to_dotted_string()),
                    &self.loc,
                )?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::Import(path_idx),
                        self.loc.clone(),
                    )
                    .write_opcode_and_source_info(
                        CompilerOpcode::PushRuntimeValue(BuiltinValueId::ThisModule),
                        self.loc.clone(),
                    )
                    .write_opcode_and_source_info(CompilerOpcode::LiftModule, self.loc.clone());
            }
        }

        Ok(())
    }
}
