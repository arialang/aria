// SPDX-License-Identifier: Apache-2.0

use rustc_data_structures::fx::FxHashMap;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Symbol(u32);

#[derive(Default)]
pub struct Interner {
    map: FxHashMap<String, Symbol>,
    strings: Vec<String>,
}

pub enum InternError {
    TooManySymbols,
}

impl Interner {
    pub fn intern(&mut self, s: &str) -> Result<Symbol, InternError> {
        if let Some(&sym) = self.map.get(s) {
            return Ok(sym);
        }

        if self.strings.len() >= u32::MAX as usize {
            return Err(InternError::TooManySymbols);
        }

        let sym = Symbol(self.strings.len() as u32);
        self.strings.push(s.to_owned());
        self.map.insert(self.strings[sym.0 as usize].clone(), sym);

        Ok(sym)
    }

    pub fn resolve(&self, sym: Symbol) -> Option<&str> {
        self.strings.get(sym.0 as usize).map(|s| s.as_str())
    }
}
