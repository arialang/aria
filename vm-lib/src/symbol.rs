// SPDX-License-Identifier: Apache-2.0

use rustc_data_structures::fx::FxHashMap;
use thiserror::Error;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct Symbol(pub u32);

pub const INTERNED_OP_IMPL_CALL: Symbol = Symbol(0);

pub const INTERNED_OP_IMPL_EQUALS: Symbol = Symbol(1);

pub const INTERNED_OP_IMPL_READ_INDEX: Symbol = Symbol(2);
pub const INTERNED_OP_IMPL_WRITE_INDEX: Symbol = Symbol(3);

pub const INTERNED_OP_IMPL_ADD: Symbol = Symbol(4);
pub const INTERNED_OP_IMPL_RADD: Symbol = Symbol(5);

pub const INTERNED_OP_IMPL_SUB: Symbol = Symbol(6);
pub const INTERNED_OP_IMPL_RSUB: Symbol = Symbol(7);

pub const INTERNED_OP_IMPL_MUL: Symbol = Symbol(8);
pub const INTERNED_OP_IMPL_RMUL: Symbol = Symbol(9);

pub const INTERNED_OP_IMPL_DIV: Symbol = Symbol(10);
pub const INTERNED_OP_IMPL_RDIV: Symbol = Symbol(11);

pub const INTERNED_OP_IMPL_REM: Symbol = Symbol(12);
pub const INTERNED_OP_IMPL_RREM: Symbol = Symbol(13);

pub const INTERNED_OP_IMPL_LSHIFT: Symbol = Symbol(14);
pub const INTERNED_OP_IMPL_RLSHIFT: Symbol = Symbol(15);

pub const INTERNED_OP_IMPL_RSHIFT: Symbol = Symbol(16);
pub const INTERNED_OP_IMPL_RRSHIFT: Symbol = Symbol(17);

pub const INTERNED_OP_IMPL_BWAND: Symbol = Symbol(18);
pub const INTERNED_OP_IMPL_RBWAND: Symbol = Symbol(19);

pub const INTERNED_OP_IMPL_BWOR: Symbol = Symbol(20);
pub const INTERNED_OP_IMPL_RBWOR: Symbol = Symbol(21);

pub const INTERNED_OP_IMPL_XOR: Symbol = Symbol(22);
pub const INTERNED_OP_IMPL_RXOR: Symbol = Symbol(23);

pub const INTERNED_OP_IMPL_LT: Symbol = Symbol(24);
pub const INTERNED_OP_IMPL_GT: Symbol = Symbol(25);

pub const INTERNED_OP_IMPL_LTEQ: Symbol = Symbol(26);
pub const INTERNED_OP_IMPL_GTEQ: Symbol = Symbol(27);

pub const INTERNED_OP_PRETTYPRINT: Symbol = Symbol(28);

pub struct Interner {
    map: FxHashMap<String, Symbol>,
    strings: Vec<String>,
}

#[derive(Clone, Error, PartialEq, Eq, Debug)]
pub enum InternError {
    #[error("too many symbols have been interned")]
    TooManySymbols,
}

impl Default for Interner {
    fn default() -> Self {
        let mut this = Self {
            map: FxHashMap::default(),
            strings: Vec::new(),
        };

        // do not alter the order of these initial interned symbols
        // without altering the corresponding constants above
        assert!(this.intern("_op_impl_call").unwrap() == INTERNED_OP_IMPL_CALL);
        assert!(this.intern("_op_impl_equals").unwrap() == INTERNED_OP_IMPL_EQUALS);
        assert!(this.intern("_op_impl_read_index").unwrap() == INTERNED_OP_IMPL_READ_INDEX);
        assert!(this.intern("_op_impl_write_index").unwrap() == INTERNED_OP_IMPL_WRITE_INDEX);
        assert!(this.intern("_op_impl_add").unwrap() == INTERNED_OP_IMPL_ADD);
        assert!(this.intern("_op_impl_radd").unwrap() == INTERNED_OP_IMPL_RADD);
        assert!(this.intern("_op_impl_sub").unwrap() == INTERNED_OP_IMPL_SUB);
        assert!(this.intern("_op_impl_rsub").unwrap() == INTERNED_OP_IMPL_RSUB);
        assert!(this.intern("_op_impl_mul").unwrap() == INTERNED_OP_IMPL_MUL);
        assert!(this.intern("_op_impl_rmul").unwrap() == INTERNED_OP_IMPL_RMUL);
        assert!(this.intern("_op_impl_div").unwrap() == INTERNED_OP_IMPL_DIV);
        assert!(this.intern("_op_impl_rdiv").unwrap() == INTERNED_OP_IMPL_RDIV);
        assert!(this.intern("_op_impl_rem").unwrap() == INTERNED_OP_IMPL_REM);
        assert!(this.intern("_op_impl_rrem").unwrap() == INTERNED_OP_IMPL_RREM);
        assert!(this.intern("_op_impl_lshift").unwrap() == INTERNED_OP_IMPL_LSHIFT);
        assert!(this.intern("_op_impl_rlshift").unwrap() == INTERNED_OP_IMPL_RLSHIFT);
        assert!(this.intern("_op_impl_rshift").unwrap() == INTERNED_OP_IMPL_RSHIFT);
        assert!(this.intern("_op_impl_rrshift").unwrap() == INTERNED_OP_IMPL_RRSHIFT);
        assert!(this.intern("_op_impl_bwand").unwrap() == INTERNED_OP_IMPL_BWAND);
        assert!(this.intern("_op_impl_rbwand").unwrap() == INTERNED_OP_IMPL_RBWAND);
        assert!(this.intern("_op_impl_bwor").unwrap() == INTERNED_OP_IMPL_BWOR);
        assert!(this.intern("_op_impl_rbwor").unwrap() == INTERNED_OP_IMPL_RBWOR);
        assert!(this.intern("_op_impl_xor").unwrap() == INTERNED_OP_IMPL_XOR);
        assert!(this.intern("_op_impl_rxor").unwrap() == INTERNED_OP_IMPL_RXOR);
        assert!(this.intern("_op_impl_lt").unwrap() == INTERNED_OP_IMPL_LT);
        assert!(this.intern("_op_impl_gt").unwrap() == INTERNED_OP_IMPL_GT);
        assert!(this.intern("_op_impl_lteq").unwrap() == INTERNED_OP_IMPL_LTEQ);
        assert!(this.intern("_op_impl_gteq").unwrap() == INTERNED_OP_IMPL_GTEQ);
        assert!(this.intern("prettyprint").unwrap() == INTERNED_OP_PRETTYPRINT);

        this
    }
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
