// SPDX-License-Identifier: Apache-2.0
use crate::ast::prettyprint::PrettyPrintable;

#[derive(Default)]
pub struct PrintoutAccumulator {
    buffer: String,
    depth: usize,
}

impl PrintoutAccumulator {
    pub(crate) fn indent_more(mut self) -> Self {
        self.depth += 1;
        self
    }

    pub(crate) fn indent_less(mut self) -> Self {
        if self.depth > 0 {
            self.depth -= 1;
        }
        self
    }

    pub(crate) fn write(mut self, text: &str) -> Self {
        if self.depth == 0 || text.find('\n').is_none() {
            self.buffer.push_str(text);
        } else {
            for ch in text.chars() {
                self.buffer.push(ch);
                if ch == '\n' {
                    self.buffer.push_str(&" ".repeat(self.depth * 4));
                }
            }
        }
        self
    }

    pub(crate) fn write_indented_list<T: PrettyPrintable + std::fmt::Debug>(
        self,
        values: &[T],
        prefix: &str,
        sep: &str,
        suffix: &str,
    ) -> Self {
        let mut this = self.indent_more().write(prefix);
        let mut first = true;
        for value in values {
            if first {
                first = false;
            } else {
                this = this.write(sep);
            }
            this = value.prettyprint(this);
        }
        this.indent_less().write(suffix)
    }

    pub(crate) fn write_separated_list<T: PrettyPrintable + std::fmt::Debug>(
        self,
        values: &[T],
        sep: &str,
    ) -> Self {
        let mut first = true;
        let mut this = self;
        for value in values {
            if first {
                first = false;
            } else {
                this = this.write(sep);
            }
            this = value.prettyprint(this);
        }
        this
    }

    pub fn value(&self) -> String {
        self.buffer.clone()
    }
}

impl<'a, T: PrettyPrintable> std::ops::Shl<&'a T> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, item: &'a T) -> Self::Output {
        item.prettyprint(self)
    }
}

impl<'a> std::ops::Shl<&'a str> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, text: &'a str) -> Self::Output {
        self.write(text)
    }
}

impl std::ops::Shl<String> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, text: String) -> Self::Output {
        self.write(&text)
    }
}

impl std::ops::Shl<&i64> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: &i64) -> Self::Output {
        self.write(&n.to_string())
    }
}

impl std::ops::Shl<f64> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: f64) -> Self::Output {
        self.write(&n.to_string())
    }
}

impl std::ops::Shl<usize> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: usize) -> Self::Output {
        self.write(&n.to_string())
    }
}

impl std::ops::Shl<u32> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: u32) -> Self::Output {
        self.write(&n.to_string())
    }
}

impl std::ops::Shl<u16> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: u16) -> Self::Output {
        self.write(&n.to_string())
    }
}

impl std::ops::Shl<u8> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: u8) -> Self::Output {
        self.write(&n.to_string())
    }
}

impl std::ops::Shl<&usize> for PrintoutAccumulator {
    type Output = Self;

    fn shl(self, n: &usize) -> Self::Output {
        self.write(&format!("{n}"))
    }
}

impl<'a, T> std::ops::Shl<&'a Option<T>> for PrintoutAccumulator
where
    T: PrettyPrintable,
{
    type Output = Self;

    fn shl(self, val: &'a Option<T>) -> Self::Output {
        if let Some(val) = val {
            val.prettyprint(self)
        } else {
            self
        }
    }
}
