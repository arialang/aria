// SPDX-License-Identifier: Apache-2.0
use crate::{
    ast::{
        Expression, LogOperation, SourceBuffer, TryUnwrapExpression,
        derive::Derive,
        prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    },
    grammar::Rule,
};

impl Derive for TryUnwrapExpression {
    fn from_parse_tree(p: pest::iterators::Pair<'_, Rule>, source: &SourceBuffer) -> Self {
        assert!(p.as_rule() == Rule::try_unwrap_expr);
        let loc = From::from(&p.as_span());
        let mut inner = p.into_inner();
        let left = LogOperation::from_parse_tree(inner.next().unwrap(), source);
        let right = Expression::from_parse_tree(inner.next().unwrap(), source);
        TryUnwrapExpression {
            loc: source.pointer(loc),
            left: Box::new(left),
            right: Box::new(right),
        }
    }
}

impl PrettyPrintable for TryUnwrapExpression {
    fn prettyprint(&self, buffer: PrintoutAccumulator) -> PrintoutAccumulator {
        self.left.prettyprint(buffer) << " ?? " << self.right.as_ref()
    }
}
