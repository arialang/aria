// SPDX-License-Identifier: Apache-2.0
use rustc_data_structures::fx::FxHashMap;
use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Symbol(pub u32);

impl std::fmt::Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Symbol({})", self.0)
    }
}

#[derive(Default)]
pub struct Interner {
    map: FxHashMap<String, Symbol>,
    strings: Vec<String>,
}

#[derive(Clone, Error, PartialEq, Eq, Debug)]
pub enum InternError {
    #[error("too many symbols have been interned")]
    TooManySymbols,
}

impl Interner {
    pub fn intern(&mut self, s: &str) -> Result<Symbol, InternError> {
        if let Some(&sym) = self.map.get(s) {
            return Ok(sym);
        }

        let id = self.strings.len();
        if id >= u32::MAX as usize {
            return Err(InternError::TooManySymbols);
        }

        let s = s.to_string();

        let sym = Symbol(id as u32);
        self.strings.push(s.clone());
        self.map.insert(s, sym);
        Ok(sym)
    }

    pub fn resolve(&self, sym: Symbol) -> Option<&str> {
        self.strings.get(sym.0 as usize).map(|s| s.as_str())
    }
}
