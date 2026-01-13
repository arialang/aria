// SPDX-License-Identifier: Apache-2.0
use crate::do_compile::{CompilationResult, CompileNode, CompileParams};

impl<'a> CompileNode<'a> for aria_parser::ast::Expression {
    fn do_compile(&self, params: &'a mut CompileParams) -> CompilationResult {
        match self {
            Self::LogOperation(lo) => lo.do_compile(params),
            Self::LambdaFunction(lf) => lf.do_compile(params),
            Self::TernaryExpression(te) => te.do_compile(params),
            Self::TryUnwrapExpression(te) => te.do_compile(params),
        }
    }
}
