// SPDX-License-Identifier: Apache-2.0

use aria_parser::ast::{DeclarationId, Identifier};

use crate::{
    builder::{compiler_opcodes::CompilerOpcode, func::FunctionBuilder},
    constant_value::{CompiledCodeObject, ConstantValue},
    do_compile::{
        CompilationError, CompilationResult, CompileNode, CompileParams, ControlFlowTargets,
        emit_args_at_target,
    },
    scope::CompilationScope,
};

impl<'a> CompileNode<'a> for aria_parser::ast::MethodDecl {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        let f_scope = CompilationScope::function(params.scope);
        let cflow = ControlFlowTargets::default();
        let mut writer = FunctionBuilder::default();
        let mut c_params = CompileParams {
            module: params.module,
            scope: &f_scope,
            writer: &mut writer,
            cflow: &cflow,
            options: params.options,
        };

        let this_arg = From::from(&DeclarationId {
            loc: self.loc.clone(),
            name: Identifier {
                loc: self.loc.clone(),
                value: match self.access {
                    aria_parser::ast::MethodAccess::Instance => "this",
                    aria_parser::ast::MethodAccess::Type => "This",
                }
                .to_owned(),
            },
            ty: None,
        });
        let argc = emit_args_at_target(&[this_arg], &self.args, &[], &mut c_params)?;

        self.body.do_compile(&mut c_params)?;
        self.return_unit_value(&mut c_params, &self.loc)?;

        let frame_size = c_params.scope.as_function_root().unwrap().num_locals();

        let co = match writer.write(&params.module.constants, params.options) {
            Ok(c) => c,
            Err(er) => {
                return Err(CompilationError {
                    loc: self.loc.clone(),
                    reason: er,
                });
            }
        };
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
        params
            .writer
            .get_current_block()
            .write_opcode_and_source_info(CompilerOpcode::Push(cco_idx), self.loc.clone());

        Ok(())
    }
}
