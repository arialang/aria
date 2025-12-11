// SPDX-License-Identifier: Apache-2.0

use std::{cell::RefCell, collections::HashSet, rc::Rc};

use aria_parser::ast::SourcePointer;

use crate::{
    bc_writer::BytecodeWriter,
    builder::{compiler_opcodes::CompilerOpcode, func::FunctionBuilder},
    constant_value::ConstantValues,
    line_table::LineTable,
};

pub(crate) struct BasicBlockEntry {
    pub op: CompilerOpcode,
    pub src: Option<SourcePointer>,
}

impl BasicBlockEntry {
    fn byte_size(&self) -> usize {
        self.op.byte_size()
    }

    fn to_vm_opcode(&self, parent: &FunctionBuilder) -> haxby_opcodes::Opcode {
        self.op.to_vm_opcode(parent)
    }
}

// TODO: this should really not be visible outside of block.rs
pub(crate) struct BasicBlockImpl {
    name: String,
    id: usize,
    pub(crate) writer: RefCell<Vec<BasicBlockEntry>>,
}

impl BasicBlockImpl {
    pub(crate) fn new(name: &str, id: usize) -> Self {
        Self {
            name: name.to_owned(),
            id,
            writer: RefCell::new(Default::default()),
        }
    }
}

#[derive(Clone)]
pub struct BasicBlock {
    pub(crate) imp: Rc<BasicBlockImpl>,
}

impl PartialEq for BasicBlock {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for BasicBlock {}

#[derive(Default)]
pub(crate) struct LocalValuesAccess {
    pub(crate) reads: HashSet<u8>,
    pub(crate) writes: HashSet<u8>,
}

impl LocalValuesAccess {
    pub(crate) fn calculate_unused_locals(&self) -> HashSet<u8> {
        self.writes.difference(&self.reads).cloned().collect()
    }
}

impl BasicBlock {
    pub(crate) fn new(name: &str, id: usize) -> Self {
        Self {
            imp: Rc::new(BasicBlockImpl::new(name, id)),
        }
    }

    pub fn name(&self) -> &str {
        &self.imp.name
    }

    pub fn id(&self) -> usize {
        self.imp.id
    }

    #[deprecated(note = "use write_opcode_and_source_info")]
    pub fn write_opcode(&self, op: CompilerOpcode) -> &Self {
        let bbe = BasicBlockEntry { op, src: None };
        self.imp.writer.borrow_mut().push(bbe);
        self
    }

    pub fn write_opcode_and_source_info(&self, op: CompilerOpcode, src: SourcePointer) -> &Self {
        let bbe = BasicBlockEntry { op, src: Some(src) };
        self.imp.writer.borrow_mut().push(bbe);
        self
    }

    pub fn len(&self) -> usize {
        self.imp.writer.borrow().len()
    }

    pub fn byte_size(&self) -> usize {
        self.imp
            .writer
            .borrow()
            .iter()
            .map(|o| o.byte_size())
            .sum::<usize>()
    }

    pub fn is_empty(&self) -> bool {
        self.imp.writer.borrow().is_empty()
    }

    pub fn is_terminal(&self) -> bool {
        let br = self.imp.writer.borrow();
        for src_op in br.as_slice() {
            if src_op.op.is_terminal() {
                return true;
            }
        }

        false
    }

    fn replace_double_jump(&self) -> bool {
        let mut any = false;

        let mut br = self.imp.writer.borrow_mut();
        for i in 0..br.len() {
            if let CompilerOpcode::Jump(dest) = &br[i].op {
                let dest = dest.clone();
                if dest.imp.id == self.imp.id {
                    continue;
                }
                if dest.is_empty() {
                    continue;
                }
                let dest_br = dest.imp.writer.borrow();
                if let CompilerOpcode::Jump(final_dest) = &dest_br[0].op {
                    br[i].op = CompilerOpcode::Jump(final_dest.clone());
                    any = true;
                }
            }
        }

        any
    }

    fn optimize_true_false(&self, cv: &ConstantValues) {
        let mut br = self.imp.writer.borrow_mut();
        for i in 0..br.len() {
            if let CompilerOpcode::ReadNamed(idx) = &br[i].op
                && let Some(crate::constant_value::ConstantValue::String(x)) = cv.get(*idx as usize)
            {
                if x == "true" {
                    br[i].op = CompilerOpcode::PushTrue;
                } else if x == "false" {
                    br[i].op = CompilerOpcode::PushFalse;
                }
            }
        }
    }

    fn optimize_redundant_conditional_jumps(&self) {
        let mut br = self.imp.writer.borrow_mut();
        let mut i = 0;
        while i + 1 < br.len() {
            match (&br[i].op, &br[i + 1].op) {
                (CompilerOpcode::PushTrue, CompilerOpcode::JumpTrue(target))
                | (CompilerOpcode::PushFalse, CompilerOpcode::JumpFalse(target)) => {
                    br[i].op = CompilerOpcode::Jump(target.clone());
                    br.remove(i + 1);
                    continue;
                }
                (CompilerOpcode::PushTrue, CompilerOpcode::JumpFalse(_))
                | (CompilerOpcode::PushFalse, CompilerOpcode::JumpTrue(_)) => {
                    br[i].op = CompilerOpcode::Nop;
                    br.remove(i + 1);
                    continue;
                }
                _ => {}
            }

            i += 1;
        }
    }

    fn remove_instructions_after_terminal(&self) {
        let mut br = self.imp.writer.borrow_mut();
        for i in 0..br.len() {
            if br[i].op.is_terminal() {
                while br.len() != i + 1 {
                    br.remove(i + 1);
                }
                return;
            }
        }
    }

    fn remove_redundant_local_reads(&self) {
        let mut br = self.imp.writer.borrow_mut();
        if br.len() < 2 {
            return;
        }

        for i in 0..br.len() - 1 {
            if let CompilerOpcode::ReadLocal(x) = br[i].op {
                let mut j = i + 1;
                while j < br.len() {
                    match br[j].op {
                        CompilerOpcode::ReadLocal(y) => {
                            if x != y {
                                break;
                            } else {
                                br[j].op = CompilerOpcode::Dup;
                            }
                        }
                        _ => {
                            break;
                        }
                    }
                    j += 1;
                }
            }
        }
    }

    fn remove_redundant_named_reads(&self) {
        let mut br = self.imp.writer.borrow_mut();
        if br.len() < 2 {
            return;
        }

        for i in 0..br.len() - 1 {
            if let CompilerOpcode::ReadNamed(x) = br[i].op {
                let mut j = i + 1;
                while j < br.len() {
                    match br[j].op {
                        CompilerOpcode::ReadNamed(y) => {
                            if x != y {
                                break;
                            } else {
                                br[j].op = CompilerOpcode::Dup;
                            }
                        }
                        _ => {
                            break;
                        }
                    }
                    j += 1;
                }
            }
        }
    }

    fn remove_store_load_sequence(&self) {
        if self.len() < 2 {
            return;
        }

        let mut br = self.imp.writer.borrow_mut();

        for i in 0..br.len() - 1 {
            if let CompilerOpcode::WriteLocal(x) = br[i].op
                && let CompilerOpcode::ReadLocal(y) = br[i + 1].op
                && x == y
            {
                br[i].op = CompilerOpcode::Dup;
                br[i + 1].op = CompilerOpcode::WriteLocal(x);
            }
        }
    }

    fn remove_nop_instructions(&self) {
        let mut br = self.imp.writer.borrow_mut();
        br.retain(|x| !matches!(x.op, CompilerOpcode::Nop));
    }

    fn remove_push_pop_pairs(&self) {
        if self.len() < 2 {
            return;
        }

        let mut br = self.imp.writer.borrow_mut();

        for i in 0..br.len() - 1 {
            if let (
                CompilerOpcode::Push0
                | CompilerOpcode::Push1
                | CompilerOpcode::PushFalse
                | CompilerOpcode::PushTrue
                | CompilerOpcode::Push(_)
                | CompilerOpcode::PushBuiltinTy(_)
                | CompilerOpcode::Dup,
                CompilerOpcode::Pop,
            ) = (&br[i].op, &br[i + 1].op)
            {
                br[i].op = CompilerOpcode::Nop;
                br[i + 1].op = CompilerOpcode::Nop;
            }
        }
    }

    pub(crate) fn run_optimize_passes(&self, cv: &ConstantValues) {
        self.optimize_true_false(cv);
        self.optimize_redundant_conditional_jumps();
        self.remove_redundant_local_reads();
        self.remove_redundant_named_reads();
        self.remove_store_load_sequence();
        self.remove_instructions_after_terminal();
        self.remove_nop_instructions();
        self.remove_push_pop_pairs();
        self.remove_nop_instructions();
        while self.replace_double_jump() {}
    }

    pub(crate) fn drop_unused_locals(&self, values: &HashSet<u8>) {
        let mut br = self.imp.writer.borrow_mut();

        for i in 0..br.len() {
            match br[i].op {
                CompilerOpcode::ReadLocal(x) => {
                    assert!(!values.contains(&x));
                }
                CompilerOpcode::TypedefLocal(x) => {
                    if values.contains(&x) {
                        br[i].op = CompilerOpcode::Pop;
                    }
                }
                CompilerOpcode::WriteLocal(x) => {
                    if values.contains(&x) {
                        br[i].op = CompilerOpcode::Pop;
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn calculate_locals_access(&self, dest: &mut LocalValuesAccess) {
        let br = self.imp.writer.borrow();
        for i in 0..br.len() {
            match br[i].op {
                CompilerOpcode::ReadLocal(x) | CompilerOpcode::StoreUplevel(x) => {
                    dest.reads.insert(x);
                }
                CompilerOpcode::WriteLocal(x) => {
                    dest.writes.insert(x);
                }
                CompilerOpcode::TypedefLocal(x) => {
                    if i > 0 {
                        if let CompilerOpcode::PushBuiltinTy(x) = br[i - 1].op
                            && x == 1
                        {
                            dest.writes.insert(x);
                        } else {
                            dest.reads.insert(x);
                            dest.writes.insert(x);
                        }
                    } else {
                        // this is quite odd, as there would have to be something else defining
                        // the type of the local on the stack, but just keep going for sake of completeness
                        dest.writes.insert(x);
                    }
                }
                _ => {}
            }
        }
    }

    pub(crate) fn write(&self, parent: &FunctionBuilder, dest: &mut BytecodeWriter) {
        let br = self.imp.writer.borrow();
        for src_op in br.as_slice() {
            dest.write_opcode(&src_op.to_vm_opcode(parent));
        }
    }

    pub(crate) fn write_line_table(
        &self,
        parent: &FunctionBuilder,
        offset: u16,
        line_table: &LineTable,
    ) {
        let mut cur_offset = offset;
        let br = self.imp.writer.borrow();
        for src_op in br.as_slice() {
            let dst_op = src_op.to_vm_opcode(parent);
            if let Some(src) = &src_op.src {
                line_table.insert(cur_offset - 1_u16, src.clone());
            }
            cur_offset += dst_op.byte_size() as u16;
        }
    }
}

impl std::fmt::Display for BasicBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let br = self.imp.writer.borrow();
        writeln!(f, "BasicBlock {}:", self.imp.name)?;
        for src_op in br.as_slice() {
            writeln!(f, "  {}", src_op.op)?;
        }

        Ok(())
    }
}
