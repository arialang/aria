// SPDX-License-Identifier: Apache-2.0
pub const OPCODE_NOP: u8 = 0;
pub const OPCODE_PUSH: u8 = 1;
pub const OPCODE_PUSH_0: u8 = 2;
pub const OPCODE_PUSH_1: u8 = 3;
pub const OPCODE_PUSH_TRUE: u8 = 4;
pub const OPCODE_PUSH_FALSE: u8 = 5;
pub const OPCODE_PUSH_BUILTIN_TYPE: u8 = 6;
pub const OPCODE_PUSH_RUNTIME_VALUE: u8 = 7;
pub const OPCODE_POP: u8 = 8;
pub const OPCODE_DUP: u8 = 9;
pub const OPCODE_SWAP: u8 = 10;
pub const OPCODE_COPY: u8 = 11;
// ..
pub const OPCODE_ADD: u8 = 20;
pub const OPCODE_SUB: u8 = 21;
pub const OPCODE_MUL: u8 = 22;
pub const OPCODE_DIV: u8 = 23;
pub const OPCODE_REM: u8 = 24;
pub const OPCODE_NEG: u8 = 25;
pub const OPCODE_SHL: u8 = 26;
pub const OPCODE_SHR: u8 = 27;
// ...
pub const OPCODE_READ_LOCAL: u8 = 30;
pub const OPCODE_WRITE_LOCAL: u8 = 31;
pub const OPCODE_TYPEDEF_LOCAL: u8 = 32;
pub const OPCODE_READ_NAMED: u8 = 33;
pub const OPCODE_WRITE_NAMED: u8 = 34;
pub const OPCODE_TYPEDEF_NAMED: u8 = 35;
pub const OPCODE_READ_INDEX: u8 = 36;
pub const OPCODE_WRITE_INDEX: u8 = 37;
pub const OPCODE_READ_ATTRIBUTE: u8 = 38;
pub const OPCODE_WRITE_ATTRIBUTE: u8 = 39;
pub const OPCODE_READ_UPLEVEL: u8 = 40;
// ...
pub const OPCODE_EQ: u8 = 50;
pub const OPCODE_LT: u8 = 51;
pub const OPCODE_GT: u8 = 52;
pub const OPCODE_LTE: u8 = 53;
pub const OPCODE_GTE: u8 = 54;
pub const OPCODE_ISA: u8 = 55;
pub const OPCODE_LOGICAL_AND: u8 = 56;
pub const OPCODE_LOGICAL_OR: u8 = 57;
pub const OPCODE_XOR: u8 = 58;
pub const OPCODE_NOT: u8 = 59;
pub const OPCODE_BITWISE_AND: u8 = 60;
pub const OPCODE_BITWISE_OR: u8 = 61;
pub const OPCODE_JUMP: u8 = 62;
pub const OPCODE_JUMP_TRUE: u8 = 63;
pub const OPCODE_JUMP_FALSE: u8 = 64;
pub const OPCODE_JUMP_CONDITIONALLY: u8 = 65;
pub const OPCODE_JUMP_IF_ARG_SUPPLIED: u8 = 66;
// ...
pub const OPCODE_TRY_ENTER: u8 = 72;
pub const OPCODE_TRY_EXIT: u8 = 73;
pub const OPCODE_THROW: u8 = 74;
pub const OPCODE_CALL: u8 = 75;
pub const OPCODE_RETURN: u8 = 76;
pub const OPCODE_RETURN_UNIT: u8 = 77;
// ...
pub const OPCODE_BUILD_LIST: u8 = 80;
pub const OPCODE_BUILD_FUNCTION: u8 = 81;
pub const OPCODE_STORE_UPLEVEL: u8 = 82;
pub const OPCODE_BUILD_STRUCT: u8 = 83;
pub const OPCODE_BUILD_ENUM: u8 = 84;
pub const OPCODE_BUILD_MIXIN: u8 = 85;
pub const OPCODE_BIND_CASE: u8 = 87;
pub const OPCODE_INCLUDE_MIXIN: u8 = 88;
pub const OPCODE_NEW_ENUM_VAL: u8 = 89;
pub const OPCODE_ENUM_CHECK_IS_CASE: u8 = 90;
pub const OPCODE_ENUM_TRY_EXTRACT_PAYLOAD: u8 = 91;
pub const OPCODE_TRY_UNWRAP_PROTOCOL: u8 = 92;
// ..
pub const OPCODE_READ_ATTRIBUTE_SYMBOL: u8 = 100;
pub const OPCODE_WRITE_ATTRIBUTE_SYMBOL: u8 = 101;
// ...
pub const OPCODE_IMPORT: u8 = 250;
pub const OPCODE_LIFT_MODULE: u8 = 251;
pub const OPCODE_LOAD_DYLIB: u8 = 252;
pub const OPCODE_ASSERT: u8 = 253;
pub const OPCODE_HALT: u8 = 254;

#[rustfmt::skip]
pub mod function_attribs {
    pub const FUNC_IS_METHOD:            u8 = 1_u8 << 0;
    pub const METHOD_ATTRIBUTE_TYPE:     u8 = 1_u8 << 1;
    pub const FUNC_ACCEPTS_VARARG:       u8 = 1_u8 << 2;
}

#[allow(unused_imports)]
use function_attribs::*;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, enum_as_inner::EnumAsInner)]
pub enum BuiltinValueId {
    ThisModule = 0,
}

impl BuiltinValueId {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuiltinValueId::ThisModule => "ThisModule",
        }
    }
}

impl TryFrom<u8> for BuiltinValueId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BuiltinValueId::ThisModule),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, enum_as_inner::EnumAsInner)]
pub enum BuiltinTypeId {
    Any = 0,
    Module = 1,
    Unit = 2,
    Unimplemented = 3,
    Maybe = 4,
    Result = 5,
    Int = 6,
    String = 7,
    RuntimeError = 8,
    Bool = 9,
    Float = 10,
    List = 11,
    Type = 12,
}

impl BuiltinTypeId {
    pub fn to_u8(&self) -> u8 {
        *self as u8
    }

    pub fn last() -> Self {
        BuiltinTypeId::Type
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuiltinTypeId::Any => "Any",
            BuiltinTypeId::Module => "Module",
            BuiltinTypeId::Int => "Int",
            BuiltinTypeId::List => "List",
            BuiltinTypeId::String => "String",
            BuiltinTypeId::Bool => "Bool",
            BuiltinTypeId::Maybe => "Maybe",
            BuiltinTypeId::Float => "Float",
            BuiltinTypeId::Unimplemented => "Unimplemented",
            BuiltinTypeId::RuntimeError => "RuntimeError",
            BuiltinTypeId::Unit => "Unit",
            BuiltinTypeId::Result => "Result",
            BuiltinTypeId::Type => "Type",
        }
    }
}

impl TryFrom<u8> for BuiltinTypeId {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(BuiltinTypeId::Any),
            1 => Ok(BuiltinTypeId::Module),
            2 => Ok(BuiltinTypeId::Unit),
            3 => Ok(BuiltinTypeId::Unimplemented),
            4 => Ok(BuiltinTypeId::Maybe),
            5 => Ok(BuiltinTypeId::Result),
            6 => Ok(BuiltinTypeId::Int),
            7 => Ok(BuiltinTypeId::String),
            8 => Ok(BuiltinTypeId::RuntimeError),
            9 => Ok(BuiltinTypeId::Bool),
            10 => Ok(BuiltinTypeId::Float),
            11 => Ok(BuiltinTypeId::List),
            12 => Ok(BuiltinTypeId::Type),
            _ => Err(()),
        }
    }
}

#[rustfmt::skip]
pub mod enum_case_attribs {
    pub const CASE_HAS_PAYLOAD:            u8 = 1_u8 << 0;
}

#[allow(unused_imports)]
use enum_case_attribs::*;

#[rustfmt::skip]
pub mod try_unwrap_protocol_mode {
    pub const PROPAGATE_ERROR:  u8 = 1;
    pub const ASSERT_ERROR:     u8 = 2;
}

#[allow(unused_imports)]
use try_unwrap_protocol_mode::*;

#[derive(Clone, Copy)]
pub enum Opcode {
    Nop,
    Push(u16),
    Push0,
    Push1,
    PushTrue,
    PushFalse,
    PushBuiltinTy(BuiltinTypeId),
    PushRuntimeValue(BuiltinValueId),
    Pop,
    Dup,
    Swap,
    Copy(u8),
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Neg,
    ShiftLeft,
    ShiftRight,
    Not,
    Equal,
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
    ReadLocal(u8),
    WriteLocal(u8),
    TypedefLocal(u8),
    ReadNamed(u16),
    WriteNamed(u16),
    TypedefNamed(u16),
    ReadIndex(u8),
    WriteIndex(u8),
    ReadAttribute(u16),
    WriteAttribute(u16),
    ReadAttributeSymbol(u32),
    WriteAttributeSymbol(u32),
    ReadUplevel(u8),
    LogicalAnd,
    LogicalOr,
    Xor,
    BitwiseAnd,
    BitwiseOr,
    JumpTrue(u16),
    JumpFalse(u16),
    Jump(u16),
    JumpConditionally(u16, u16),
    JumpIfArgSupplied(u8, u16),
    Call(u8),
    Return,
    ReturnUnit,
    TryEnter(u16),
    TryExit,
    Throw,
    BuildList(u32),
    BuildFunction(u8),
    StoreUplevel(u8),
    BuildStruct,
    BuildEnum,
    BuildMixin,
    BindCase(u8, u16),
    IncludeMixin,
    NewEnumVal(u8, u16),
    EnumCheckIsCase(u16),
    EnumTryExtractPayload,
    TryUnwrapProtocol(u8),
    Isa,
    Import(u16),
    LiftModule,
    LoadDylib(u16),
    Assert(u16),
    Halt,
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nop => write!(f, "NOP"),
            Self::Push(arg0) => write!(f, "PUSH @{arg0}"),
            Self::Push0 => write!(f, "PUSH_0"),
            Self::Push1 => write!(f, "PUSH_1"),
            Self::PushTrue => write!(f, "PUSH_T"),
            Self::PushFalse => write!(f, "PUSH_F"),
            Self::PushBuiltinTy(arg0) => write!(f, "PUSH_BUILTIN_TY {}", arg0.name()),
            Self::PushRuntimeValue(arg0) => write!(f, "PUSH_RUNTIME_VAL {}", arg0.name()),
            Self::Pop => write!(f, "POP"),
            Self::Dup => write!(f, "DUP"),
            Self::Swap => write!(f, "SWAP"),
            Self::Copy(n) => write!(f, "COPY -{n}"),
            Self::Add => write!(f, "ADD"),
            Self::Sub => write!(f, "SUB"),
            Self::Mul => write!(f, "MUL"),
            Self::Div => write!(f, "DIV"),
            Self::Rem => write!(f, "REM"),
            Self::Equal => write!(f, "EQ"),
            Self::Neg => write!(f, "NEG"),
            Self::ShiftLeft => write!(f, "SHL"),
            Self::ShiftRight => write!(f, "SHR"),
            Self::Not => write!(f, "NOT"),
            Self::ReadLocal(arg0) => write!(f, "READ_LOCAL {arg0}"),
            Self::WriteLocal(arg0) => write!(f, "WRITE_LOCAL {arg0}"),
            Self::TypedefLocal(arg0) => write!(f, "TYPEDEF_LOCAL {arg0}"),
            Self::ReadNamed(arg0) => write!(f, "READ_NAMED @{arg0}"),
            Self::WriteNamed(arg0) => write!(f, "WRITE_NAMED @{arg0}"),
            Self::TypedefNamed(arg0) => write!(f, "TYPEDEF_NAMED @{arg0}"),
            Self::ReadIndex(arg0) => write!(f, "READ_INDEX {arg0}"),
            Self::WriteIndex(arg0) => write!(f, "WRITE_INDEX {arg0}"),
            Self::ReadAttribute(arg0) => write!(f, "READ_ATTRIB @{arg0}"),
            Self::WriteAttribute(arg0) => write!(f, "WRITE_ATTRIB @{arg0}"),
            Self::ReadAttributeSymbol(arg0) => write!(f, "READ_ATTRIB_SYM #{arg0}"),
            Self::WriteAttributeSymbol(arg0) => write!(f, "WRITE_ATTRIB_SYM #{arg0}"),
            Self::ReadUplevel(arg0) => write!(f, "READ_UPLEVEL {arg0}"),
            Self::LogicalAnd => write!(f, "ANDL"),
            Self::LogicalOr => write!(f, "ORL"),
            Self::Xor => write!(f, "XOR"),
            Self::BitwiseAnd => write!(f, "ANDB"),
            Self::BitwiseOr => write!(f, "ORB"),
            Self::LessThan => write!(f, "LT"),
            Self::GreaterThan => write!(f, "GT"),
            Self::LessThanEqual => write!(f, "LTE"),
            Self::GreaterThanEqual => write!(f, "GTE"),
            Self::JumpTrue(arg0) => write!(f, "JUMP_TRUE {arg0}"),
            Self::JumpFalse(arg0) => write!(f, "JUMP_FALSE {arg0}"),
            Self::Jump(arg0) => write!(f, "JUMP {arg0}"),
            Self::JumpConditionally(arg0, arg1) => write!(f, "JUMP_CONDITIONALLY {arg0} {arg1}"),
            Self::JumpIfArgSupplied(arg0, arg1) => write!(f, "JUMP_IF_ARG_SUPPLIED {arg0} {arg1}"),
            Self::Call(arg0) => write!(f, "CALL {arg0}"),
            Self::Return => write!(f, "RETURN"),
            Self::ReturnUnit => write!(f, "RETURN_UNIT"),
            Self::TryEnter(arg0) => write!(f, "ENTER_TRY {arg0}"),
            Self::TryExit => write!(f, "EXIT_TRY"),
            Self::Throw => write!(f, "THROW"),
            Self::BuildList(arg0) => write!(f, "BUILD_LIST {arg0}"),
            Self::BuildFunction(arg0) => write!(f, "BUILD_FUNC {arg0}"),
            Self::StoreUplevel(arg0) => write!(f, "STORE_UPLEVEL {arg0}"),
            Self::BuildStruct => write!(f, "BUILD_STRUCT"),
            Self::BuildEnum => write!(f, "BUILD_ENUM"),
            Self::BuildMixin => write!(f, "BUILD_MIXIN"),
            Self::BindCase(arg0, arg1) => write!(f, "BIND_C {arg0} @{arg1}"),
            Self::IncludeMixin => write!(f, "INCLUDE_MIXIN"),
            Self::NewEnumVal(arg0, arg1) => write!(f, "NEW_ENUM_VAL {arg0} @{arg1}"),
            Self::EnumCheckIsCase(arg0) => write!(f, "ENUM_CHECK_IS_CASE @{arg0}"),
            Self::EnumTryExtractPayload => write!(f, "ENUM_TRY_EXTRACT_PAYLOAD"),
            Self::TryUnwrapProtocol(mode) => write!(f, "TRY_UNWRAP_PROTOCOL {mode}"),
            Self::Isa => write!(f, "ISA"),
            Self::Import(arg0) => write!(f, "IMPORT @{arg0}"),
            Self::LiftModule => write!(f, "LIFT_MODULE"),
            Self::LoadDylib(arg0) => write!(f, "LOAD_DYLIB @{arg0}"),
            Self::Assert(arg0) => write!(f, "ASSERT @{arg0}"),
            Self::Halt => write!(f, "HALT"),
        }
    }
}
