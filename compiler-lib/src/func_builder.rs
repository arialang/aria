// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use aria_parser::ast::SourcePointer;
use haxby_opcodes::Opcode;

use crate::{
    CompilationOptions, bc_writer::BytecodeWriter, builder::compiler_opcodes::CompilerOpcode,
    constant_value::ConstantValues, line_table::LineTable,
};

struct BasicBlockEntry {
    op: CompilerOpcode,
    src: Option<SourcePointer>,
}

impl From<CompilerOpcode> for BasicBlockEntry {
    fn from(op: CompilerOpcode) -> Self {
        Self { op, src: None }
    }
}

impl BasicBlockEntry {
    fn byte_size(&self) -> usize {
        self.op.byte_size()
    }

    fn to_vm_opcode(&self, parent: &FunctionBuilder) -> Opcode {
        self.op.to_vm_opcode(parent)
    }
}

pub struct BasicBlock {
    pub(crate) name: String,
    pub(crate) id: usize,
    writer: RefCell<Vec<BasicBlockEntry>>,
}

struct LocalValuesAccess {
    reads: HashSet<u8>,
    writes: HashSet<u8>,
}

impl LocalValuesAccess {
    fn calculate_unused_locals(&self) -> HashSet<u8> {
        self.writes.difference(&self.reads).cloned().collect()
    }
}

impl BasicBlock {
    fn new(name: &str, id: usize) -> Self {
        Self {
            name: name.to_owned(),
            id,
            writer: RefCell::new(Default::default()),
        }
    }

    #[deprecated(note = "use write_opcode_and_source_info")]
    pub fn write_opcode(&self, op: CompilerOpcode) -> &Self {
        self.writer.borrow_mut().push(op.into());
        self
    }

    pub fn write_opcode_and_source_info(&self, op: CompilerOpcode, src: SourcePointer) -> &Self {
        let bbe = BasicBlockEntry { op, src: Some(src) };
        self.writer.borrow_mut().push(bbe);
        self
    }

    pub fn len(&self) -> usize {
        self.writer.borrow().len()
    }

    pub fn byte_size(&self) -> usize {
        self.writer
            .borrow()
            .iter()
            .map(|o| o.byte_size())
            .sum::<usize>()
    }

    pub fn is_empty(&self) -> bool {
        self.writer.borrow().is_empty()
    }

    pub fn is_terminal(&self) -> bool {
        let br = self.writer.borrow();
        for src_op in br.as_slice() {
            if src_op.op.is_terminal() {
                return true;
            }
        }

        false
    }

    fn replace_double_jump(&self) -> bool {
        let mut any = false;

        let mut br = self.writer.borrow_mut();
        for i in 0..br.len() {
            if let CompilerOpcode::Jump(dest) = &br[i].op {
                let dest = dest.clone();
                if dest.id == self.id {
                    continue;
                }
                if dest.is_empty() {
                    continue;
                }
                let dest_br = dest.writer.borrow();
                if let CompilerOpcode::Jump(final_dest) = &dest_br[0].op {
                    br[i].op = CompilerOpcode::Jump(final_dest.clone());
                    any = true;
                }
            }
        }

        any
    }

    fn optimize_true_false(&self, cv: &ConstantValues) {
        let mut br = self.writer.borrow_mut();
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
        let mut br = self.writer.borrow_mut();
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
        let mut br = self.writer.borrow_mut();
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
        let mut br = self.writer.borrow_mut();
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
        let mut br = self.writer.borrow_mut();
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
        let mut br = self.writer.borrow_mut();
        if br.len() < 2 {
            return;
        }

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
        let mut br = self.writer.borrow_mut();
        br.retain(|x| !matches!(x.op, CompilerOpcode::Nop));
    }

    fn remove_push_pop_pairs(&self) {
        let mut br = self.writer.borrow_mut();
        if br.len() < 2 {
            return;
        }

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

    fn run_optimize_passes(&self, cv: &ConstantValues) {
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

    fn drop_unused_locals(&self, values: &HashSet<u8>) {
        let mut br = self.writer.borrow_mut();

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

    fn calculate_locals_access(&self, dest: &mut LocalValuesAccess) {
        let br = self.writer.borrow();
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

    fn write(&self, parent: &FunctionBuilder, dest: &mut BytecodeWriter) {
        let br = self.writer.borrow();
        for src_op in br.as_slice() {
            dest.write_opcode(&src_op.to_vm_opcode(parent));
        }
    }

    fn write_line_table(&self, parent: &FunctionBuilder, offset: u16, line_table: &LineTable) {
        let mut cur_offset = offset;
        let br = self.writer.borrow();
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
        let br = self.writer.borrow();
        writeln!(f, "BasicBlock {}:", self.name)?;
        for src_op in br.as_slice() {
            writeln!(f, "  {}", src_op.op)?;
        }

        Ok(())
    }
}

pub struct FunctionBuilder {
    blocks: Vec<Rc<BasicBlock>>,
    names: HashSet<String>,
    current: Rc<BasicBlock>,
    bb_id: usize,
    line_table: LineTable,
}

impl Default for FunctionBuilder {
    fn default() -> Self {
        let mut this = Self {
            blocks: Vec::new(),
            names: HashSet::new(),
            current: Rc::new(BasicBlock::new("entry", 0)),
            bb_id: 1,
            line_table: Default::default(),
        };
        this.blocks.push(this.current.clone());
        this.names.insert(this.current.name.clone());
        this
    }
}

impl FunctionBuilder {
    pub fn try_get_block(&self, name: &str) -> Option<Rc<BasicBlock>> {
        for blk in &self.blocks {
            if blk.name == name {
                return Some(blk.clone());
            }
        }

        None
    }

    pub fn get_block(&self, name: &str) -> Rc<BasicBlock> {
        self.try_get_block(name).expect("block is missing")
    }

    fn uniq_name(&self, name: &str) -> String {
        let mut name = String::from(name);
        while self.names.contains(&name) {
            name += "_";
        }

        name
    }

    fn make_new_block(&mut self, name: &str) -> Rc<BasicBlock> {
        assert!(!self.names.contains(name));

        let blk = Rc::new(BasicBlock::new(name, self.bb_id));
        self.bb_id += 1;
        blk
    }

    pub fn insert_block_after(&mut self, name: &str, target: &Rc<BasicBlock>) -> Rc<BasicBlock> {
        let name = self.uniq_name(name);
        let blk = self.make_new_block(&name);
        let mut inserted = false;

        for i in 0..self.blocks.len() {
            let blk_i = &self.blocks[i];
            if blk_i.id == target.id {
                if i + 1 >= self.blocks.len() {
                    self.blocks.push(blk.clone());
                } else {
                    self.blocks.insert(i + 1, blk.clone());
                }
                inserted = true;
                break;
            }
        }

        if !inserted {
            self.blocks.push(blk.clone());
        }

        self.names.insert(name);
        blk
    }

    pub fn append_block_at_end(&mut self, name: &str) -> Rc<BasicBlock> {
        let name = self.uniq_name(name);
        let blk = self.make_new_block(&name);

        self.blocks.push(blk.clone());
        self.names.insert(name);
        blk
    }

    pub fn set_current_block(&mut self, blk: Rc<BasicBlock>) {
        self.current = blk;
    }

    pub fn get_current_block(&self) -> Rc<BasicBlock> {
        self.current.clone()
    }

    pub fn offset_of_block(&self, blk: &Rc<BasicBlock>) -> Option<u16> {
        let mut count = 0;
        for next in &self.blocks {
            if Rc::ptr_eq(next, blk) {
                return Some((count + 1) as u16);
            } else {
                count += next.byte_size();
            }
        }
        None
    }

    fn find_orphaned_blocks(&self) -> HashSet<usize> {
        let mut orphans = HashSet::<usize>::default();

        for blk in &self.blocks {
            if blk.id != 0 {
                orphans.insert(blk.id);
            }
        }

        for blk in &self.blocks {
            let br = blk.writer.borrow();
            for src_op in br.as_slice() {
                if let Some(dst) = src_op.op.is_jump_instruction() {
                    orphans.remove(&dst.id);
                }
            }
        }

        orphans
    }

    fn remove_block_with_id(&mut self, id: usize) -> bool {
        for i in 0..self.blocks.len() {
            if self.blocks[i].id == id {
                self.blocks.remove(i);
                return true;
            }
        }

        false
    }

    fn run_optimize_passes(&mut self, cv: &ConstantValues) {
        let orphans = self.find_orphaned_blocks();
        for orphan_id in &orphans {
            assert!(self.remove_block_with_id(*orphan_id));
        }

        let locals_access = self.calculate_locals_access();
        let unused_locals = locals_access.calculate_unused_locals();

        for blk in &self.blocks {
            if !unused_locals.is_empty() {
                blk.drop_unused_locals(&unused_locals);
            }
            blk.run_optimize_passes(cv);
        }
    }

    fn calculate_locals_access(&self) -> LocalValuesAccess {
        let mut dest = LocalValuesAccess {
            reads: HashSet::new(),
            writes: HashSet::new(),
        };

        for blk in &self.blocks {
            blk.calculate_locals_access(&mut dest);
        }

        dest
    }

    pub fn write(
        &mut self,
        cv: &ConstantValues,
        options: &CompilationOptions,
    ) -> Result<Vec<u8>, crate::do_compile::CompilationErrorReason> {
        if options.dump_builder {
            println!("(unopt) Intermediate Representation Dump:\n{}", self);
        }
        if options.optimize {
            self.run_optimize_passes(cv);
            if options.dump_builder {
                println!("(opt) Intermediate Representation Dump:\n{}", self);
            }
        }

        let mut dest = BytecodeWriter::default();
        for blk in &self.blocks {
            assert!(blk.is_empty() || blk.is_terminal());
            blk.write(self, &mut dest);
        }

        let ret = dest.get_data();
        if ret.len() >= u16::MAX.into() {
            Err(crate::do_compile::CompilationErrorReason::FunctionBodyTooLarge)
        } else {
            Ok(ret)
        }
    }

    pub fn write_line_table(&self) -> &LineTable {
        for blk in &self.blocks {
            blk.write_line_table(self, self.offset_of_block(blk).unwrap(), &self.line_table);
        }

        &self.line_table
    }
}

impl std::fmt::Display for FunctionBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for blk in &self.blocks {
            writeln!(f, "{}", blk)?;
        }

        Ok(())
    }
}
