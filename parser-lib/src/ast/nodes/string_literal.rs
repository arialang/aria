// SPDX-License-Identifier: Apache-2.0
use crate::{
    ast::{
        SourceBuffer, StringLiteral,
        derive::Derive,
        prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    },
    grammar::Rule,
};

// TODO: process string literals in the compiler code, not the parser
// the parser has no good way to report an error, so complete the processing
// in the compiler where we can fail on an invalid escape sequence
fn process_string_literal(s: &str) -> String {
    fn process_string_escapes(s: &str) -> String {
        let mut result = String::new();
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next) = chars.peek() {
                    match next {
                        'n' => {
                            result.push('\n');
                            chars.next();
                        }
                        'r' => {
                            result.push('\r');
                            chars.next();
                        }
                        't' => {
                            result.push('\t');
                            chars.next();
                        }
                        '\\' => {
                            result.push('\\');
                            chars.next();
                        }
                        'X' | 'x' => {
                            let _ = chars.next();
                            if let (Some(high), Some(low)) = (chars.next(), chars.next())
                                && let (Some(high), Some(low)) =
                                    (high.to_digit(16), low.to_digit(16))
                            {
                                result.push((high << 4 | low) as u8 as char);
                            }
                        }
                        'U' | 'u' => {
                            let _ = chars.next();
                            if chars.peek() == Some(&'{') {
                                let _ = chars.next();
                                let mut hex_digits = String::new();
                                while let Some(&next) = chars.peek() {
                                    if next == '}' {
                                        let _ = chars.next();
                                        break;
                                    } else {
                                        hex_digits.push(chars.next().unwrap());
                                    }
                                }
                                if let Ok(codepoint) = u32::from_str_radix(&hex_digits, 16)
                                    && let Some(chr) = char::from_u32(codepoint)
                                {
                                    result.push(chr);
                                }
                            }
                        }
                        _ => {
                            result.push(c);
                        }
                    }
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    let s = &s[1..s.len() - 1];
    process_string_escapes(s)
}

impl Derive for StringLiteral {
    fn from_parse_tree(p: pest::iterators::Pair<'_, Rule>, source: &SourceBuffer) -> Self {
        assert!(p.as_rule() == Rule::str_literal);
        let loc = From::from(&p.as_span());
        Self {
            loc: source.pointer(loc),
            value: process_string_literal(p.as_str()),
        }
    }
}

impl PrettyPrintable for StringLiteral {
    fn prettyprint(&self, buffer: PrintoutAccumulator) -> PrintoutAccumulator {
        buffer.write(&self.value)
    }
}
