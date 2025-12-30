// SPDX-License-Identifier: Apache-2.0

pub struct Stack<T> {
    values: Vec<T>,
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Self { values: Vec::new() }
    }
}

impl<T> Stack<T> {
    pub fn push(&mut self, val: T) {
        self.values.push(val);
    }

    pub fn peek(&mut self) -> Option<&T> {
        self.values.last()
    }

    pub fn try_pop(&mut self) -> Option<T> {
        self.values.pop()
    }

    pub fn pop(&mut self) -> T {
        self.try_pop().expect("empty stack")
    }

    pub fn pop_count(&mut self, count: usize) -> Vec<T> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            if let Some(v) = self.try_pop() {
                result.push(v);
            } else {
                break;
            }
        }

        result
    }

    pub fn try_pop_if<F, U>(&mut self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> Option<U>,
    {
        match self.try_pop() {
            Some(t) => f(t),
            None => None,
        }
    }

    pub fn pop_if<F, U>(&mut self, f: F) -> Option<U>
    where
        F: FnOnce(T) -> Option<U>,
    {
        f(self.pop())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn peek_at_offset(&mut self, i: usize) -> Option<&T> {
        let idx = self.len() - 1 - i;
        self.values.get(idx)
    }

    pub fn contains(&self, val: &T) -> bool
    where
        T: PartialEq<T>,
    {
        self.values.iter().any(|x| val.eq(x))
    }
}

impl<T> Stack<T>
where
    T: std::fmt::Debug,
{
    pub fn dump(&self) {
        let mut first = true;
        self.values.iter().rev().for_each(|f| {
            if first {
                first = false;
                println!("--> {f:?}")
            } else {
                println!("{f:?}")
            };
        });
    }
}
