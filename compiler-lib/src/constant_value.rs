// SPDX-License-Identifier: Apache-2.0
use std::collections::HashMap;

use aria_parser::ast::SourcePointer;
use enum_as_inner::EnumAsInner;

use crate::line_table::LineTable;

#[derive(Clone, PartialEq, Eq)]
pub struct CompiledCodeObject {
    pub name: String,
    pub attribute: u8,
    pub body: Vec<u8>,
    pub required_argc: u8, // arguments that are required to call this function
    pub default_argc: u8,  // additional arguments that this function can accept
    pub loc: SourcePointer,
    pub line_table: LineTable,
    pub frame_size: u8,
}

#[derive(Clone, Copy)]
pub struct FpConst(f64);

impl FpConst {
    fn to_int(self) -> i64 {
        unsafe { std::mem::transmute(self) }
    }
    pub fn raw_value(&self) -> f64 {
        self.0
    }
}
impl PartialEq for FpConst {
    fn eq(&self, other: &Self) -> bool {
        self.to_int() == other.to_int()
    }
}
impl Eq for FpConst {}
impl std::hash::Hash for FpConst {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.to_int().hash(state)
    }
}
impl From<f64> for FpConst {
    fn from(value: f64) -> Self {
        Self(value)
    }
}

// ignore the line table and only hash on function code
impl std::hash::Hash for CompiledCodeObject {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.body.hash(state);
    }
}

#[derive(EnumAsInner, Clone, PartialEq, Eq, Hash)]
pub enum ConstantValue {
    Integer(i64),
    String(String),
    Float(FpConst),
    CompiledCodeObject(CompiledCodeObject),
}

impl std::fmt::Display for ConstantValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(arg0) => write!(f, "int:{}", *arg0),
            Self::Float(arg0) => write!(f, "fp:{}", arg0.0),
            Self::String(arg0) => write!(f, "str:\"{}\"", *arg0),
            Self::CompiledCodeObject(_) => write!(f, "compiled-code-object"),
        }
    }
}

#[derive(Default)]
pub struct ConstantValues {
    pub(crate) values: Vec<ConstantValue>,
    uniq: HashMap<ConstantValue, usize>,
}

pub enum ConstantValuesError {
    OutOfSpace,
}

impl ConstantValues {
    pub fn insert(&mut self, v: ConstantValue) -> Result<u16, ConstantValuesError> {
        if let Some(idx) = self.uniq.get(&v) {
            Ok(*idx as u16)
        } else {
            if self.values.len() == (u16::MAX as usize) {
                return Err(ConstantValuesError::OutOfSpace);
            }

            let idx = self.values.len();
            self.uniq.insert(v.clone(), idx);
            self.values.push(v);
            Ok(idx as u16)
        }
    }

    pub fn get(&self, i: usize) -> Option<ConstantValue> {
        self.values.get(i).cloned()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn values(&self) -> std::slice::Iter<'_, ConstantValue> {
        self.values.iter()
    }
}
