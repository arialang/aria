// SPDX-License-Identifier: Apache-2.0
use crate::{
    ast::{
        PostfixTermTryProtocol, SourceBuffer, TryProtocolMode,
        derive::Derive,
        prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    },
    grammar::Rule,
};

impl Derive for PostfixTermTryProtocol {
    fn from_parse_tree(p: pest::iterators::Pair<'_, Rule>, source: &SourceBuffer) -> Self {
        assert!(p.as_rule() == Rule::postfix_term_try_protocol);
        let loc = From::from(&p.as_span());
        let token_rule = p
            .into_inner()
            .next()
            .and_then(|inner| inner.into_inner().next())
            .map(|inner| inner.as_rule())
            .unwrap_or_else(|| panic!("? or ! expected"));
        let mode = match token_rule {
            Rule::postfix_term_try_protocol_return_token => TryProtocolMode::Return,
            Rule::postfix_term_try_protocol_assert_token => TryProtocolMode::Assert,
            _ => panic!("? or ! expected"),
        };

        Self {
            loc: source.pointer(loc),
            mode,
        }
    }
}

impl PrettyPrintable for PostfixTermTryProtocol {
    fn prettyprint(&self, buffer: PrintoutAccumulator) -> PrintoutAccumulator {
        buffer
            << match self.mode {
                TryProtocolMode::Assert => "!",
                TryProtocolMode::Return => "?",
            }
    }
}
