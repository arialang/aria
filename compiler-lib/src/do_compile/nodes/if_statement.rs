// SPDX-License-Identifier: Apache-2.0
use crate::{
    builder::compiler_opcodes::CompilerOpcode,
    do_compile::{CompilationResult, CompileNode, CompileParams},
};

impl<'a> CompileNode<'a> for aria_parser::ast::IfStatement {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        let after_all = params
            .writer
            .append_block_at_end(&format!("after_all_{}", self.loc));

        // a trivial if is of the form
        // if foo {
        //     bar
        //}
        // in which case, we can simply emit a jump to "after all", i.e. compile as
        // <foo>
        // JUMP_FALSE <after_all>
        // <then>...
        // JUMP <after_all> since the block needs to be terminated
        // <after_all>...
        let is_trivial_if = self.elsif.is_empty() && self.els.is_none();

        // compile the first if
        {
            self.iff.content.expression.do_compile(params)?;
            let if_true = params.writer.insert_block_after(
                &format!("if_true_{}", self.loc),
                &params.writer.get_current_block(),
            );
            let if_false = params
                .writer
                .insert_block_after(&format!("if_false_{}", self.loc), &if_true);
            if is_trivial_if {
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::JumpFalse(after_all.clone()),
                        self.iff.content.loc.clone(),
                    );
            } else {
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::JumpConditionally(if_true.clone(), if_false.clone()),
                        self.iff.content.loc.clone(),
                    );
                params.writer.set_current_block(if_true);
            }
            self.iff.content.then.do_compile(params)?;
            params
                .writer
                .get_current_block()
                .write_opcode_and_source_info(
                    CompilerOpcode::Jump(after_all.clone()),
                    self.iff.content.loc.clone(),
                );
            params.writer.set_current_block(if_false);
        }

        // compile every elsif
        {
            for elsif in &self.elsif {
                elsif.content.expression.do_compile(params)?;
                let if_true = params.writer.insert_block_after(
                    &format!("if_true_{}", self.loc),
                    &params.writer.get_current_block(),
                );
                let if_false = params
                    .writer
                    .insert_block_after(&format!("if_false_{}", self.loc), &if_true);

                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::JumpConditionally(if_true.clone(), if_false.clone()),
                        elsif.content.then.loc.clone(),
                    );
                params.writer.set_current_block(if_true);
                elsif.content.then.do_compile(params)?;
                params
                    .writer
                    .get_current_block()
                    .write_opcode_and_source_info(
                        CompilerOpcode::Jump(after_all.clone()),
                        elsif.content.loc.clone(),
                    );
                params.writer.set_current_block(if_false);
            }
        }

        // compile the else
        {
            if let Some(els) = &self.els {
                els.then.do_compile(params)?;
            }
        }

        if !is_trivial_if {
            // jump to after all
            params
                .writer
                .get_current_block()
                .write_opcode_and_source_info(
                    CompilerOpcode::Jump(after_all.clone()),
                    self.loc.clone(),
                );
        }
        params.writer.set_current_block(after_all);

        Ok(())
    }
}
