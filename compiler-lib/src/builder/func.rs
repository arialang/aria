// SPDX-License-Identifier: Apache-2.0
use std::collections::HashSet;

use crate::{
    CompilationOptions,
    bc_writer::BytecodeWriter,
    builder::{
        block::{BasicBlock, LocalValuesAccess},
        compiler_opcodes::CompilerOpcode,
    },
    constant_value::ConstantValues,
    line_table::LineTable,
};

pub struct FunctionBuilder {
    blocks: Vec<BasicBlock>,
    names: HashSet<String>,
    current: BasicBlock,
    bb_id: usize,
    line_table: LineTable,
}

impl Default for FunctionBuilder {
    fn default() -> Self {
        let mut this = Self {
            blocks: Vec::new(),
            names: HashSet::new(),
            current: BasicBlock::new("entry", 0),
            bb_id: 1,
            line_table: Default::default(),
        };
        this.blocks.push(this.current.clone());
        this.names.insert(this.current.name().to_owned());
        this
    }
}

#[allow(dead_code)]
struct BlockEntryPoint {
    source: BasicBlock,
    dest: BasicBlock,
    op: CompilerOpcode,
    op_idx: usize,
}

impl FunctionBuilder {
    pub fn try_get_block(&self, name: &str) -> Option<BasicBlock> {
        for blk in &self.blocks {
            if blk.name() == name {
                return Some(blk.clone());
            }
        }

        None
    }

    pub fn get_block(&self, name: &str) -> BasicBlock {
        self.try_get_block(name).expect("block is missing")
    }

    fn uniq_name(&self, name: &str) -> String {
        let mut name = String::from(name);
        while self.names.contains(&name) {
            name += "_";
        }

        name
    }

    fn make_new_block(&mut self, name: &str) -> BasicBlock {
        assert!(!self.names.contains(name));

        let blk = BasicBlock::new(name, self.bb_id);
        self.bb_id += 1;
        blk
    }

    pub fn insert_block_after(&mut self, name: &str, target: &BasicBlock) -> BasicBlock {
        let name = self.uniq_name(name);
        let blk = self.make_new_block(&name);
        let mut inserted = false;

        for i in 0..self.blocks.len() {
            let blk_i = &self.blocks[i];
            if blk_i.id() == target.id() {
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

    pub fn append_block_at_end(&mut self, name: &str) -> BasicBlock {
        let name = self.uniq_name(name);
        let blk = self.make_new_block(&name);

        self.blocks.push(blk.clone());
        self.names.insert(name);
        blk
    }

    pub fn set_current_block(&mut self, blk: BasicBlock) {
        self.current = blk;
    }

    pub fn get_current_block(&self) -> BasicBlock {
        self.current.clone()
    }

    pub fn offset_of_block(&self, blk: &BasicBlock) -> Option<u16> {
        let mut count = 0;
        for next in &self.blocks {
            if next == blk {
                return Some((count + 1) as u16);
            } else {
                count += next.byte_size();
            }
        }
        None
    }

    fn get_block_entrypoints(&self, blk: &BasicBlock) -> Vec<BlockEntryPoint> {
        let mut dests = Vec::<BlockEntryPoint>::new();

        for src_blk in &self.blocks {
            let br = src_blk.imp.writer.borrow();
            for (op_idx, src_op) in br.as_slice().iter().enumerate() {
                if let Some(dst) = src_op.op.is_jump_instruction()
                    && dst.id() == blk.id()
                {
                    dests.push(BlockEntryPoint {
                        source: src_blk.clone(),
                        dest: dst,
                        op: src_op.op.clone(),
                        op_idx,
                    });
                }
            }
        }

        dests
    }

    fn find_orphaned_blocks(&self) -> HashSet<usize> {
        let mut orphans = HashSet::<usize>::default();

        for blk in &self.blocks {
            if blk.id() != 0 {
                let entrypoints = self.get_block_entrypoints(blk);
                if entrypoints.is_empty() {
                    orphans.insert(blk.id());
                }
            }
        }

        orphans
    }

    fn remove_block_with_id(&mut self, id: usize) -> bool {
        for i in 0..self.blocks.len() {
            if self.blocks[i].id() == id {
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
        let mut dest = LocalValuesAccess::default();

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
