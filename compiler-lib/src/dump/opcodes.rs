// SPDX-License-Identifier: Apache-2.0
use aria_parser::ast::prettyprint::printout_accumulator::PrintoutAccumulator;
use haxby_opcodes::Opcode;

use crate::module::CompiledModule;

fn const_best_repr(module: &CompiledModule, idx: u16) -> String {
    match module.load_indexed_const(idx) {
        Some(s) => s.to_string(),
        None => format!("invalid const @{idx}"),
    }
}

fn try_protocol_mode_to_str(id: u8) -> &'static str {
    match id {
        haxby_opcodes::try_unwrap_protocol_mode::PROPAGATE_ERROR => "RETURN",
        haxby_opcodes::try_unwrap_protocol_mode::ASSERT_ERROR => "ASSERT",
        _ => "Unknown",
    }
}

pub fn opcode_prettyprint(
    opcode: Opcode,
    module: &CompiledModule,
    buffer: PrintoutAccumulator,
) -> PrintoutAccumulator {
    match opcode {
        Opcode::Push(idx) => {
            buffer << "PUSH(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::PushBuiltinTy(n) => {
            buffer << "PUSH_BUILTIN_TY(" << n.to_u8() << ") [" << n.name() << "]"
        }
        Opcode::PushRuntimeValue(n) => {
            buffer << "PUSH_RUNTIME_VAL(" << n.to_u8() << ") [" << n.name() << "]"
        }
        Opcode::ReadNamed(idx) => {
            buffer << "READ_NAMED(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::WriteNamed(idx) => {
            buffer << "WRITE_NAMED(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::TypedefNamed(idx) => {
            buffer << "TYPEDEF_NAMED(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::ReadAttribute(idx) => {
            buffer << "READ_ATTRIB(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::WriteAttribute(idx) => {
            buffer << "WRITE_ATTRIB(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::BindMethod(arg, idx) => {
            buffer
                << "BIND_METHOD("
                << arg
                << ",@"
                << idx
                << ") ["
                << const_best_repr(module, idx)
                << "]"
        }
        Opcode::BindCase(arg, idx) => {
            buffer
                << "BIND_CASE("
                << arg
                << ",@"
                << idx
                << ") ["
                << const_best_repr(module, idx)
                << "]"
        }
        Opcode::NewEnumVal(flag, idx) => {
            buffer
                << "NEW_ENUM_VAL("
                << flag
                << ",@"
                << idx
                << ") ["
                << const_best_repr(module, idx)
                << "]"
        }
        Opcode::EnumCheckIsCase(idx) => {
            buffer << "ENUM_CHECK_IS_CASE(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::Import(idx) => {
            buffer << "IMPORT(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::LoadDylib(idx) => {
            buffer << "LOAD_DYLIB(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::Assert(idx) => {
            buffer << "ASSERT(@" << idx << ") [" << const_best_repr(module, idx) << "]"
        }
        Opcode::TryUnwrapProtocol(mode) => {
            buffer << "TRY_UNWRAP_PROTOCOL " << try_protocol_mode_to_str(mode)
        }
        Opcode::Nop
        | Opcode::Push0
        | Opcode::Push1
        | Opcode::PushTrue
        | Opcode::PushFalse
        | Opcode::Pop
        | Opcode::Dup
        | Opcode::Swap
        | Opcode::Copy(_)
        | Opcode::Add
        | Opcode::Sub
        | Opcode::Mul
        | Opcode::Div
        | Opcode::Rem
        | Opcode::Neg
        | Opcode::ShiftLeft
        | Opcode::ShiftRight
        | Opcode::Not
        | Opcode::Equal
        | Opcode::ReadLocal(_)
        | Opcode::WriteLocal(_)
        | Opcode::TypedefLocal(_)
        | Opcode::ReadIndex(_)
        | Opcode::WriteIndex(_)
        | Opcode::ReadUplevel(_)
        | Opcode::ReadAttributeInterned(_)
        | Opcode::LogicalAnd
        | Opcode::LogicalOr
        | Opcode::Xor
        | Opcode::BitwiseAnd
        | Opcode::BitwiseOr
        | Opcode::GreaterThan
        | Opcode::LessThan
        | Opcode::GreaterThanEqual
        | Opcode::LessThanEqual
        | Opcode::JumpTrue(_)
        | Opcode::JumpFalse(_)
        | Opcode::Jump(_)
        | Opcode::JumpIfArgSupplied(..)
        | Opcode::Call(_)
        | Opcode::Return
        | Opcode::ReturnUnit
        | Opcode::TryEnter(_)
        | Opcode::TryExit
        | Opcode::Throw
        | Opcode::BuildList(_)
        | Opcode::BuildFunction(_)
        | Opcode::StoreUplevel(_)
        | Opcode::BuildStruct
        | Opcode::BuildEnum
        | Opcode::BuildMixin
        | Opcode::IncludeMixin
        | Opcode::EnumTryExtractPayload
        | Opcode::Isa
        | Opcode::LiftModule
        | Opcode::Halt => buffer << opcode.to_string(),
    }
}
