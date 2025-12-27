// SPDX-License-Identifier: Apache-2.0
use crate::do_compile::{
    CompilationResult, CompileNode, CompileParams, MixinIncludeDecl, StructEntry,
    emit_type_members_compile,
};

impl<'a> CompileNode<'a> for aria_parser::ast::ExtensionDecl {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        self.target.do_compile(params)?;

        // Inject mixin includes for each item in the inherits list
        if !self.inherits.is_empty() {
            let mut new_body = vec![];
            for mixin_expr in &self.inherits {
                new_body.push(StructEntry::MixinInclude(Box::new(MixinIncludeDecl {
                    loc: self.loc.clone(),
                    what: mixin_expr.clone(),
                })));
            }
            new_body.extend_from_slice(&self.body);

            emit_type_members_compile(&new_body, params, true)
        } else {
            emit_type_members_compile(&self.body, params, true)
        }
    }
}
