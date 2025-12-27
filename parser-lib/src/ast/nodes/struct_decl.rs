// SPDX-License-Identifier: Apache-2.0
use crate::{
    ast::{
        Expression, Identifier, SourceBuffer, StructDecl, StructEntry,
        derive::Derive,
        prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    },
    grammar::Rule,
};

impl Derive for StructDecl {
    fn from_parse_tree(p: pest::iterators::Pair<'_, Rule>, source: &SourceBuffer) -> Self {
        assert!(p.as_rule() == Rule::struct_decl);
        let loc = From::from(&p.as_span());
        let mut inner = p.into_inner();
        let name = Identifier::from_parse_tree(inner.next().expect("need identifier"), source);
        let inherits = if let Some(next) = inner.peek() {
            if next.as_rule() == Rule::mixin_list {
                let mixin_list = inner.next().unwrap();
                mixin_list
                    .into_inner()
                    .map(|expr| Expression::from_parse_tree(expr, source))
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };
        let mut body = vec![];
        for next in inner {
            let next = StructEntry::from_parse_tree(next, source);
            body.push(next);
        }
        Self {
            loc: source.pointer(loc),
            name,
            inherits,
            body,
        }
    }
}

impl PrettyPrintable for StructDecl {
    fn prettyprint(&self, buffer: PrintoutAccumulator) -> PrintoutAccumulator {
        (buffer << "struct " << &self.name).write_indented_list(&self.body, "{\n", "\n", "\n}")
    }
}
