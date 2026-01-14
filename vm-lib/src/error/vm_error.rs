// SPDX-License-Identifier: Apache-2.0

use aria_compiler::bc_reader::DecodeError;
use aria_parser::ast::{SourcePointer, prettyprint::printout_accumulator::PrintoutAccumulator};
use haxby_opcodes::Opcode;
use thiserror::Error;

use crate::{
    error::backtrace::Backtrace, opcodes::prettyprint::opcode_prettyprint,
    runtime_module::RuntimeModule,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum SymbolKind {
    Identifier,
    Case,
}

impl std::fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Identifier => write!(f, "identifier"),
            SymbolKind::Case => write!(f, "case"),
        }
    }
}

#[derive(Clone, Error, PartialEq, Eq, Debug)]
pub enum VmErrorReason {
    #[error("assertion failed: {0}")]
    AssertFailed(String),

    #[error("'{0}' is a circular import reference")]
    CircularImport(String),

    #[error("division by zero")]
    DivisionByZero,

    #[error("enum value has no payload")]
    EnumWithoutPayload,

    #[error("runtime stack is empty")]
    EmptyStack,

    #[error("index {0} out of bounds")]
    IndexOutOfBounds(usize),

    #[error("cannot import module at path '{0}': {1}")]
    ImportNotAvailable(String, String),

    #[error("instruction cannot be fully decoded")]
    IncompleteInstruction,

    #[error("bytecode ended without an explicit terminal instruction")]
    UnterminatedBytecode,

    #[error("invalid binding")]
    InvalidBinding,

    #[error("control instruction invalid")]
    InvalidControlInstruction,

    #[error("mismatched argument count, expected {0} actual {1}")]
    MismatchedArgumentCount(usize, usize),

    #[error("unknown named identifier: '{0}'")]
    NoSuchIdentifier(String),

    #[error("'{0}' is not a valid case for this enum")]
    NoSuchCase(String),

    #[error("unknown {1} symbol: '{0}'")]
    NoSuchSymbol(u32, SymbolKind),

    #[error("unknown module constant value: '{0}'")]
    NoSuchModuleConstant(u16),

    #[error("operation failed: {0}")]
    OperationFailed(String),

    #[error("unexpected value type")]
    UnexpectedType,

    #[error("VM execution is not a valid state")]
    UnexpectedVmState,

    #[error("The main function must have 0, 1 or variable arguments")]
    InvalidMainSignature,

    #[error("uplevel {0} not available")]
    UplevelOutOfBounds(usize),

    #[error("{0} is not a known opcode")]
    UnknownOpcode(u8),

    #[error("{1} is not a valid operand for opcode {0}")]
    InvalidVmOperand(u8, u8),

    #[error("bytecode exceeds maximum allowed size")]
    BytecodeTooLarge,

    #[error("too many symbols have been interned")]
    TooManyInternedSymbols,

    #[error("VM execution halted")]
    VmHalted,
}

impl From<DecodeError> for VmErrorReason {
    fn from(value: DecodeError) -> Self {
        match value {
            DecodeError::DataTooLarge => VmErrorReason::BytecodeTooLarge,
            DecodeError::EndOfStream => VmErrorReason::UnterminatedBytecode,
            DecodeError::InsufficientData => VmErrorReason::IncompleteInstruction,
            DecodeError::UnknownOpcode(n) => VmErrorReason::UnknownOpcode(n),
            DecodeError::UnknownOperand(n, m) => VmErrorReason::InvalidVmOperand(n, m),
        }
    }
}

#[derive(Clone)]
pub struct VmError {
    pub reason: VmErrorReason,
    pub opcode: Option<Opcode>,
    pub loc: Option<SourcePointer>,
    pub backtrace: Box<Backtrace>,
}

impl VmError {
    pub fn prettyprint(&self, module: Option<RuntimeModule>) -> String {
        let mut poa = PrintoutAccumulator::default();
        poa = poa << "vm error: " << self.reason.to_string();
        if let Some(opcode) = self.opcode
            && let Some(m) = module
        {
            poa = {
                let ropc = crate::opcodes::prettyprint::RuntimeOpcodePrinter {
                    globals: None,
                    module: Some(&m),
                };
                opcode_prettyprint(opcode, &ropc, poa)
            };
        }
        if let Some(loc) = &self.loc {
            poa = poa << " at " << loc.to_string();
        }

        poa.value()
    }
}

impl std::fmt::Debug for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.prettyprint(None))
    }
}

impl From<VmErrorReason> for VmError {
    fn from(reason: VmErrorReason) -> Self {
        Self {
            reason,
            opcode: None,
            loc: None,
            backtrace: Default::default(),
        }
    }
}
