// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::function_attribs::FUNC_ACCEPTS_VARARG;

use crate::{
    builder::{compiler_opcodes::CompilerOpcode, func::FunctionBuilder},
    constant_value::{CompiledCodeObject, ConstantValue},
    do_compile::{
        CompilationError, CompilationResult, CompileNode, CompileParams, ControlFlowTargets,
        emit_args_at_target,
    },
};

impl<'a> CompileNode<'a> for aria_parser::ast::FunctionDecl {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        let cflow = ControlFlowTargets::default();
        let mut writer = FunctionBuilder::default();
        let mut c_params = CompileParams {
            module: params.module,
            scope: params.scope,
            writer: &mut writer,
            cflow: &cflow,
            options: params.options,
        };

        let argc = emit_args_at_target(&[], &self.args, &[], &mut c_params)?;

        self.body.do_compile(&mut c_params)?;
        self.return_unit_value(&mut c_params, &self.loc)?;

        let co = match writer.write(&params.module.constants, params.options) {
            Ok(c) => c,
            Err(er) => {
                return Err(CompilationError {
                    loc: self.loc.clone(),
                    reason: er,
                });
            }
        };
        let frame_size = params.scope.as_function_root().unwrap().num_locals();
        let line_table = writer.write_line_table().clone();
        let cco = CompiledCodeObject {
            name: self.name.value.clone(),
            body: co,
            required_argc: argc.required_args,
            default_argc: argc.default_args,
            loc: self.loc.clone(),
            line_table,
            frame_size,
        };
        let cco_idx =
            self.insert_const_or_fail(params, ConstantValue::CompiledCodeObject(cco), &self.loc)?;
        let a = if self.args.vararg {
            FUNC_ACCEPTS_VARARG
        } else {
            0_u8
        };
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(CompilerOpcode::Push(cco_idx), self.loc.clone())
            .write_opcode_and_source_info(CompilerOpcode::BuildFunction(a), self.loc.clone());

        for uplv in params
            .scope
            .as_function_root()
            .unwrap()
            .uplevels
            .borrow()
            .iter()
        {
            params
                .writer
                .get_current_block()
                .write_opcode_and_source_info(
                    CompilerOpcode::StoreUplevel(uplv.idx_in_uplevel),
                    self.loc.clone(),
                );
        }

        params
            .scope
            .get_module_scope()
            .unwrap()
            .emit_untyped_define(
                &self.name.value,
                &mut params.module.constants,
                params.writer.get_current_block(),
                self.loc.clone(),
            )?;
        Ok(())
    }
}
