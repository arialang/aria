// SPDX-License-Identifier: Apache-2.0
use aria_parser::ast::{ImportFromStatement, ImportPath, ParsedModule};
use haxby_opcodes::BuiltinValueId;

use crate::{
    builder::compiler_opcodes::CompilerOpcode,
    constant_value::{CompiledCodeObject, ConstantValue},
    do_compile::{CompilationError, CompilationResult, CompileNode, CompileParams},
};

macro_rules! collate_error_if_any {
    {$expression: expr, $errors: expr} => {
        if let Err(e) = $expression {
            $errors.push(e);
        }
    }
}

impl<'a> CompileNode<'a, (), Vec<CompilationError>> for ParsedModule {
    fn do_compile(
        &self,
        params: &'a mut CompileParams,
    ) -> CompilationResult<(), Vec<CompilationError>> {
        let mut errors = vec![];

        if !self
            .flags
            .flags
            .contains(&aria_parser::ast::ModuleFlag::NoStandardLibrary)
        {
            let import_core_statement = ImportFromStatement {
                loc: self.loc.clone(),
                what: aria_parser::ast::ImportTarget::All,
                from: ImportPath::from_dotted_string(self.loc.clone(), "aria.core.builtin"),
            };
            collate_error_if_any!(import_core_statement.do_compile(params), errors);
        }

        for pf in &self.entries {
            match pf {
                aria_parser::ast::TopLevelEntry::ValDeclStatement(v) => {
                    collate_error_if_any!(v.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::WriteOpEqStatement(w) => {
                    collate_error_if_any!(w.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::AssignStatement(a) => {
                    collate_error_if_any!(a.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::FunctionDecl(f) => {
                    let f_scope = params.scope.function();
                    let mut f_params = CompileParams {
                        module: params.module,
                        scope: &f_scope,
                        writer: params.writer,
                        cflow: params.cflow,
                        options: params.options,
                    };
                    collate_error_if_any!(f.do_compile(&mut f_params), errors)
                }
                aria_parser::ast::TopLevelEntry::StructDecl(s) => {
                    collate_error_if_any!(s.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::MixinDecl(m) => {
                    collate_error_if_any!(m.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::ExtensionDecl(e) => {
                    collate_error_if_any!(e.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::ExpressionStatement(e) => {
                    collate_error_if_any!(e.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::AssertStatement(a) => {
                    collate_error_if_any!(a.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::EnumDecl(e) => {
                    collate_error_if_any!(e.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::ImportStatement(i) => {
                    collate_error_if_any!(i.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::ImportFromStatement(i) => {
                    collate_error_if_any!(i.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::IfStatement(i) => {
                    collate_error_if_any!(i.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::MatchStatement(m) => {
                    collate_error_if_any!(m.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::WhileStatement(w) => {
                    collate_error_if_any!(w.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::ForStatement(f) => {
                    collate_error_if_any!(f.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::CodeBlock(c) => {
                    collate_error_if_any!(c.do_compile(params), errors)
                }
                aria_parser::ast::TopLevelEntry::TryBlock(t) => {
                    collate_error_if_any!(t.do_compile(params), errors)
                }
            }
        }

        for flag in &self.flags.flags {
            if let aria_parser::ast::ModuleFlag::UsesDylib(lib) = flag {
                let cidx = match self.insert_const_or_fail(
                    params,
                    ConstantValue::String(lib.clone()),
                    &self.loc,
                ) {
                    Ok(i) => i,
                    Err(e) => {
                        errors.push(e);
                        return Err(errors);
                    }
                };
                #[allow(deprecated)] // flags have no location info
                params
                    .writer
                    .get_current_block()
                    .write_opcode(CompilerOpcode::PushRuntimeValue(BuiltinValueId::ThisModule))
                    .write_opcode(CompilerOpcode::LoadDylib(cidx));
            }
        }

        #[allow(deprecated)] // no entry to ascribe this write to
        params
            .writer
            .get_current_block()
            .write_opcode(CompilerOpcode::Return);

        let co = match params
            .writer
            .write(&params.module.constants, params.options)
        {
            Ok(c) => c,
            Err(e) => {
                errors.push(CompilationError {
                    loc: self.loc.clone(),
                    reason: e,
                });
                return Err(errors);
            }
        };

        let frame_size = 0;
        let line_table = params.writer.write_line_table().clone();
        let __entry_cco = CompiledCodeObject {
            name: "__entry".to_owned(),
            body: co,
            required_argc: 0,
            default_argc: 0,
            loc: self.loc.clone(),
            line_table,
            frame_size,
        };

        if let Err(e) = self.insert_const_or_fail(
            params,
            ConstantValue::CompiledCodeObject(__entry_cco),
            &self.loc,
        ) {
            errors.push(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
