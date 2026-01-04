// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::Opcode;

#[derive(Default)]
pub(crate) struct BytecodeWriter {
    data: Vec<u8>,
}

impl BytecodeWriter {
    fn write_u8(&mut self, val: u8) -> &mut Self {
        self.data.push(val);
        self
    }

    fn write_u16(&mut self, val: u16) -> &mut Self {
        let bytes = val.to_le_bytes();
        self.write_u8(bytes[0]).write_u8(bytes[1])
    }

    fn write_u32(&mut self, val: u32) -> &mut Self {
        let bytes = val.to_le_bytes();
        self.write_u8(bytes[0])
            .write_u8(bytes[1])
            .write_u8(bytes[2])
            .write_u8(bytes[3])
    }

    pub(crate) fn write_opcode(&mut self, op: &Opcode) -> &mut Self {
        match op {
            Opcode::Nop => self.write_u8(haxby_opcodes::OPCODE_NOP),
            Opcode::Push(n) => self.write_u8(haxby_opcodes::OPCODE_PUSH).write_u16(*n),
            Opcode::Push0 => self.write_u8(haxby_opcodes::OPCODE_PUSH_0),
            Opcode::Push1 => self.write_u8(haxby_opcodes::OPCODE_PUSH_1),
            Opcode::PushTrue => self.write_u8(haxby_opcodes::OPCODE_PUSH_TRUE),
            Opcode::PushFalse => self.write_u8(haxby_opcodes::OPCODE_PUSH_FALSE),
            Opcode::PushBuiltinTy(n) => self
                .write_u8(haxby_opcodes::OPCODE_PUSH_BUILTIN_TYPE)
                .write_u8(n.to_u8()),
            Opcode::PushRuntimeValue(n) => self
                .write_u8(haxby_opcodes::OPCODE_PUSH_RUNTIME_VALUE)
                .write_u8(n.to_u8()),
            Opcode::Pop => self.write_u8(haxby_opcodes::OPCODE_POP),
            Opcode::Dup => self.write_u8(haxby_opcodes::OPCODE_DUP),
            Opcode::Swap => self.write_u8(haxby_opcodes::OPCODE_SWAP),
            Opcode::Copy(n) => self.write_u8(haxby_opcodes::OPCODE_COPY).write_u8(*n),
            Opcode::Add => self.write_u8(haxby_opcodes::OPCODE_ADD),
            Opcode::Sub => self.write_u8(haxby_opcodes::OPCODE_SUB),
            Opcode::Mul => self.write_u8(haxby_opcodes::OPCODE_MUL),
            Opcode::Div => self.write_u8(haxby_opcodes::OPCODE_DIV),
            Opcode::Rem => self.write_u8(haxby_opcodes::OPCODE_REM),
            Opcode::Neg => self.write_u8(haxby_opcodes::OPCODE_NEG),
            Opcode::ShiftLeft => self.write_u8(haxby_opcodes::OPCODE_SHL),
            Opcode::ShiftRight => self.write_u8(haxby_opcodes::OPCODE_SHR),
            Opcode::Not => self.write_u8(haxby_opcodes::OPCODE_NOT),
            Opcode::Equal => self.write_u8(haxby_opcodes::OPCODE_EQ),
            Opcode::ReadLocal(n) => self.write_u8(haxby_opcodes::OPCODE_READ_LOCAL).write_u8(*n),
            Opcode::WriteLocal(n) => self
                .write_u8(haxby_opcodes::OPCODE_WRITE_LOCAL)
                .write_u8(*n),
            Opcode::TypedefLocal(n) => self
                .write_u8(haxby_opcodes::OPCODE_TYPEDEF_LOCAL)
                .write_u8(*n),
            Opcode::ReadNamed(n) => self
                .write_u8(haxby_opcodes::OPCODE_READ_NAMED)
                .write_u16(*n),
            Opcode::WriteNamed(n) => self
                .write_u8(haxby_opcodes::OPCODE_WRITE_NAMED)
                .write_u16(*n),
            Opcode::TypedefNamed(n) => self
                .write_u8(haxby_opcodes::OPCODE_TYPEDEF_NAMED)
                .write_u16(*n),
            Opcode::ReadIndex(n) => self.write_u8(haxby_opcodes::OPCODE_READ_INDEX).write_u8(*n),
            Opcode::WriteIndex(n) => self
                .write_u8(haxby_opcodes::OPCODE_WRITE_INDEX)
                .write_u8(*n),
            Opcode::ReadAttribute(n) => self
                .write_u8(haxby_opcodes::OPCODE_READ_ATTRIBUTE)
                .write_u16(*n),
            Opcode::ReadAttributeInterned(n) => self
                .write_u8(haxby_opcodes::OPCODE_READ_ATTRIBUTE_INTERNED)
                .write_u32(*n),
            Opcode::WriteAttribute(n) => self
                .write_u8(haxby_opcodes::OPCODE_WRITE_ATTRIBUTE)
                .write_u16(*n),
            Opcode::ReadUplevel(n) => self
                .write_u8(haxby_opcodes::OPCODE_READ_UPLEVEL)
                .write_u8(*n),
            Opcode::LogicalAnd => self.write_u8(haxby_opcodes::OPCODE_LOGICAL_AND),
            Opcode::LogicalOr => self.write_u8(haxby_opcodes::OPCODE_LOGICAL_OR),
            Opcode::Xor => self.write_u8(haxby_opcodes::OPCODE_XOR),
            Opcode::BitwiseAnd => self.write_u8(haxby_opcodes::OPCODE_BITWISE_AND),
            Opcode::BitwiseOr => self.write_u8(haxby_opcodes::OPCODE_BITWISE_OR),
            Opcode::GreaterThan => self.write_u8(haxby_opcodes::OPCODE_GT),
            Opcode::LessThan => self.write_u8(haxby_opcodes::OPCODE_LT),
            Opcode::GreaterThanEqual => self.write_u8(haxby_opcodes::OPCODE_GTE),
            Opcode::LessThanEqual => self.write_u8(haxby_opcodes::OPCODE_LTE),
            Opcode::JumpTrue(n) => self.write_u8(haxby_opcodes::OPCODE_JUMP_TRUE).write_u16(*n),
            Opcode::JumpFalse(n) => self
                .write_u8(haxby_opcodes::OPCODE_JUMP_FALSE)
                .write_u16(*n),
            Opcode::Jump(n) => self.write_u8(haxby_opcodes::OPCODE_JUMP).write_u16(*n),
            Opcode::JumpIfArgSupplied(n, d) => self
                .write_u8(haxby_opcodes::OPCODE_JUMP_IF_ARG_SUPPLIED)
                .write_u8(*n)
                .write_u16(*d),
            Opcode::Call(n) => self.write_u8(haxby_opcodes::OPCODE_CALL).write_u8(*n),
            Opcode::Return => self.write_u8(haxby_opcodes::OPCODE_RETURN),
            Opcode::ReturnUnit => self.write_u8(haxby_opcodes::OPCODE_RETURN_UNIT),
            Opcode::TryEnter(n) => self.write_u8(haxby_opcodes::OPCODE_TRY_ENTER).write_u16(*n),
            Opcode::TryExit => self.write_u8(haxby_opcodes::OPCODE_TRY_EXIT),
            Opcode::Throw => self.write_u8(haxby_opcodes::OPCODE_THROW),
            Opcode::BuildList(n) => self
                .write_u8(haxby_opcodes::OPCODE_BUILD_LIST)
                .write_u32(*n),
            Opcode::BuildFunction(n) => self
                .write_u8(haxby_opcodes::OPCODE_BUILD_FUNCTION)
                .write_u8(*n),
            Opcode::StoreUplevel(n) => self
                .write_u8(haxby_opcodes::OPCODE_STORE_UPLEVEL)
                .write_u8(*n),
            Opcode::BuildStruct => self.write_u8(haxby_opcodes::OPCODE_BUILD_STRUCT),
            Opcode::BuildMixin => self.write_u8(haxby_opcodes::OPCODE_BUILD_MIXIN),
            Opcode::BuildEnum => self.write_u8(haxby_opcodes::OPCODE_BUILD_ENUM),
            Opcode::BindMethod(a, n) => self
                .write_u8(haxby_opcodes::OPCODE_BIND_METHOD)
                .write_u8(*a)
                .write_u16(*n),
            Opcode::BindCase(a, n) => self
                .write_u8(haxby_opcodes::OPCODE_BIND_CASE)
                .write_u8(*a)
                .write_u16(*n),
            Opcode::IncludeMixin => self.write_u8(haxby_opcodes::OPCODE_INCLUDE_MIXIN),
            Opcode::NewEnumVal(a, n) => self
                .write_u8(haxby_opcodes::OPCODE_NEW_ENUM_VAL)
                .write_u8(*a)
                .write_u16(*n),
            Opcode::EnumCheckIsCase(n) => self
                .write_u8(haxby_opcodes::OPCODE_ENUM_CHECK_IS_CASE)
                .write_u16(*n),
            Opcode::EnumTryExtractPayload => {
                self.write_u8(haxby_opcodes::OPCODE_ENUM_TRY_EXTRACT_PAYLOAD)
            }
            Opcode::TryUnwrapProtocol(n) => self
                .write_u8(haxby_opcodes::OPCODE_TRY_UNWRAP_PROTOCOL)
                .write_u8(*n),
            Opcode::Isa => self.write_u8(haxby_opcodes::OPCODE_ISA),
            Opcode::Import(n) => self.write_u8(haxby_opcodes::OPCODE_IMPORT).write_u16(*n),
            Opcode::LiftModule => self.write_u8(haxby_opcodes::OPCODE_LIFT_MODULE),
            Opcode::LoadDylib(n) => self
                .write_u8(haxby_opcodes::OPCODE_LOAD_DYLIB)
                .write_u16(*n),
            Opcode::Assert(n) => self.write_u8(haxby_opcodes::OPCODE_ASSERT).write_u16(*n),
            Opcode::Halt => self.write_u8(haxby_opcodes::OPCODE_HALT),
        }
    }

    pub(crate) fn get_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}
