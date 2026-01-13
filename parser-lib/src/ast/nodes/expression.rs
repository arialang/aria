// SPDX-License-Identifier: Apache-2.0
use crate::{
    ast::TernaryExpression,
    ast::{
        Expression, LambdaFunction, LogOperation, SourceBuffer, TryUnwrapExpression,
        derive::Derive,
        prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    },
    grammar::Rule,
};

impl Derive for Expression {
    fn from_parse_tree(p: pest::iterators::Pair<'_, Rule>, source: &SourceBuffer) -> Self {
        assert!(p.as_rule() == Rule::expression);
        let content = p.into_inner().next().expect("needs an atom inside");
        match content.as_rule() {
            Rule::log => Self::LogOperation(LogOperation::from_parse_tree(content, source)),
            Rule::lambda_f => {
                Self::LambdaFunction(LambdaFunction::from_parse_tree(content, source))
            }
            Rule::ternary_expr => {
                Self::TernaryExpression(TernaryExpression::from_parse_tree(content, source))
            }
            Rule::try_unwrap_expr => {
                Self::TryUnwrapExpression(TryUnwrapExpression::from_parse_tree(content, source))
            }
            _ => panic!("atom does not contain"),
        }
    }
}

impl PrettyPrintable for Expression {
    fn prettyprint(&self, buffer: PrintoutAccumulator) -> PrintoutAccumulator {
        match self {
            Expression::LogOperation(c) => c.prettyprint(buffer),
            Expression::LambdaFunction(f) => f.prettyprint(buffer),
            Expression::TernaryExpression(t) => t.prettyprint(buffer),
            Expression::TryUnwrapExpression(t) => t.prettyprint(buffer),
        }
    }
}
