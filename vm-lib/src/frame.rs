// SPDX-License-Identifier: Apache-2.0

use aria_compiler::line_table::LineTable;
use aria_parser::ast::SourcePointer;

use crate::{
    runtime_value::{RuntimeValue, function::Function, isa::IsaCheckable},
    stack::Stack,
    vm::VirtualMachine,
};

pub struct LocalVariable {
    pub val: RuntimeValue,
    pub ty: IsaCheckable,
}

impl Default for LocalVariable {
    fn default() -> Self {
        Self {
            val: RuntimeValue::Integer(From::from(0)),
            ty: IsaCheckable::any(),
        }
    }
}

#[derive(Clone)]
pub enum ControlBlock {
    Try(u16),
}

pub struct Frame {
    pub stack: Stack<RuntimeValue>,
    pub(crate) line_table: Option<LineTable>,
    pub(crate) ctrl_blocks: Stack<ControlBlock>,
    pub(crate) locals: Vec<LocalVariable>,
    pub(crate) func: Option<Function>,
    pub argc: u8,
}

impl Frame {
    pub(crate) fn drop_to_first_try(&mut self, _: &mut VirtualMachine) -> Option<u16> {
        if let Some(block) = self.ctrl_blocks.try_pop() {
            match block {
                ControlBlock::Try(x) => {
                    return Some(x);
                }
            }
        }

        None
    }
}

impl Frame {
    pub fn new_with_function(f: Function) -> Self {
        let mut this = Self::new_with_n_locals(f.frame_size());
        this.set_line_table(f.line_table());
        this.func = Some(f);
        this
    }

    pub(crate) fn new_with_n_locals(n: u8) -> Self {
        let mut this = Self {
            stack: Default::default(),
            line_table: None,
            ctrl_blocks: Default::default(),
            locals: Vec::with_capacity(n as usize),
            func: None,
            argc: 0,
        };
        for _ in 0..n {
            this.locals.push(LocalVariable::default())
        }
        this
    }

    pub(crate) fn set_argc(&mut self, argc: u8) -> &mut Self {
        self.argc = argc;
        self
    }

    pub(crate) fn set_line_table(&mut self, lt: Option<&LineTable>) -> &mut Self {
        self.line_table = lt.cloned();
        self
    }

    pub(crate) fn get_line_table(&self) -> Option<&LineTable> {
        self.line_table.as_ref()
    }

    pub fn get_line_entry_at_pos(&self, pos: u16) -> Option<SourcePointer> {
        if let Some(lt) = &self.line_table {
            lt.get(pos)
        } else {
            None
        }
    }

    pub(crate) fn reset_for_function(&mut self, f: &Function) {
        self.stack.clear();
        self.ctrl_blocks.clear();
        self.func = Some(f.clone());
        self.argc = 0;
        self.set_line_table(f.line_table());
        let locals = f.frame_size() as usize;
        self.locals.clear();
        self.locals.resize_with(locals, LocalVariable::default);
    }

    pub(crate) fn reset_for_pool(mut self) -> Self {
        self.stack.clear();
        self.ctrl_blocks.clear();
        self.locals.clear();
        self.func = None;
        self.argc = 0;
        self.line_table = None;
        self
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new_with_n_locals(0)
    }
}
