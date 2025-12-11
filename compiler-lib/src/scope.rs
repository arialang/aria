// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use aria_parser::ast::SourcePointer;
use haxby_opcodes::builtin_type_ids::BUILTIN_TYPE_ANY;

use crate::{
    builder::{block::BasicBlock, compiler_opcodes::CompilerOpcode},
    constant_value::ConstantValues,
};

trait Numeric<Output = Self> {
    fn zero() -> Output;

    fn one() -> Output;
}

impl Numeric for u8 {
    fn zero() -> u8 {
        0_u8
    }

    fn one() -> u8 {
        1_u8
    }
}

impl Numeric for u16 {
    fn zero() -> u16 {
        0_u16
    }

    fn one() -> u16 {
        1_u16
    }
}

struct IndexProviderImpl<T>
where
    T: std::ops::AddAssign<T> + Numeric + Copy,
{
    next_idx: T,
}

impl<T> Default for IndexProviderImpl<T>
where
    T: std::ops::AddAssign<T> + Numeric + Copy,
{
    fn default() -> Self {
        Self {
            next_idx: T::zero(),
        }
    }
}

impl<T> IndexProviderImpl<T>
where
    T: std::ops::AddAssign<T> + Numeric + Copy,
{
    fn next(&mut self) -> T {
        let current = self.next_idx;
        self.next_idx += T::one();
        current
    }

    fn get_max_index(&self) -> T {
        self.next_idx
    }
}

pub enum ScopeErrorReason {
    TooManyConstants,
    OverlyDeepClosure,
    NoSuchIdentifier(String),
}

pub struct ScopeError {
    pub loc: SourcePointer,
    pub reason: ScopeErrorReason,
}

pub type ScopeResult<T = ()> = Result<T, ScopeError>;

#[derive(Default)]
pub struct ModuleRootScope {
    symbols: RefCell<HashMap<String, u16>>,
}

impl ModuleRootScope {
    pub fn emit_typed_define(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        let symbol_idx = match consts.insert(crate::constant_value::ConstantValue::String(
            name.to_owned(),
        )) {
            Ok(c) => c,
            Err(_) => {
                return Err(ScopeError {
                    loc,
                    reason: ScopeErrorReason::TooManyConstants,
                });
            }
        };
        self.symbols
            .borrow_mut()
            .insert(name.to_owned(), symbol_idx);
        dest.write_opcode_and_source_info(CompilerOpcode::TypedefNamed(symbol_idx), loc);
        Ok(())
    }

    pub fn emit_write(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::WriteNamed(*existing_idx), loc);
            Ok(())
        } else {
            let symbol_idx = match consts.insert(crate::constant_value::ConstantValue::String(
                name.to_owned(),
            )) {
                Ok(c) => c,
                Err(_) => {
                    return Err(ScopeError {
                        loc,
                        reason: ScopeErrorReason::TooManyConstants,
                    });
                }
            };
            dest.write_opcode_and_source_info(CompilerOpcode::WriteNamed(symbol_idx), loc);
            Ok(())
        }
    }

    pub fn emit_read(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::ReadNamed(*existing_idx), loc);
        } else {
            let symbol_idx = match consts.insert(crate::constant_value::ConstantValue::String(
                name.to_owned(),
            )) {
                Ok(c) => c,
                Err(_) => {
                    return Err(ScopeError {
                        loc,
                        reason: ScopeErrorReason::TooManyConstants,
                    });
                }
            };
            dest.write_opcode_and_source_info(CompilerOpcode::ReadNamed(symbol_idx), loc);
        }
        Ok(())
    }

    fn resolve_uplevel_symbol(
        &self,
        _: &str,
        _: BasicBlock,
        _: SourcePointer,
        _: bool,
    ) -> ScopeResult<Option<UplevelSymbolResolution>> {
        Ok(None)
    }
}

pub struct ModuleChildScope {
    symbols: RefCell<HashMap<String, u16>>,
    parent: CompilationScope,
}

impl ModuleChildScope {
    fn new(parent: CompilationScope) -> Self {
        Self {
            symbols: Default::default(),
            parent,
        }
    }

    pub fn emit_typed_define(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        let symbol_idx = match consts.insert(crate::constant_value::ConstantValue::String(
            name.to_owned(),
        )) {
            Ok(c) => c,
            Err(_) => {
                return Err(ScopeError {
                    loc,
                    reason: ScopeErrorReason::TooManyConstants,
                });
            }
        };
        self.symbols
            .borrow_mut()
            .insert(name.to_owned(), symbol_idx);
        dest.write_opcode_and_source_info(CompilerOpcode::TypedefNamed(symbol_idx), loc);
        Ok(())
    }

    pub fn emit_write(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::WriteNamed(*existing_idx), loc);
            Ok(())
        } else {
            self.parent.emit_write(name, consts, dest, loc)
        }
    }

    pub fn emit_read(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::ReadNamed(*existing_idx), loc);
            Ok(())
        } else {
            self.parent.emit_read(name, consts, dest, loc)
        }
    }

    fn resolve_uplevel_symbol(
        &self,
        _: &str,
        _: BasicBlock,
        _: SourcePointer,
        _: bool,
    ) -> ScopeResult<Option<UplevelSymbolResolution>> {
        Ok(None)
    }
}

#[derive(Copy, Clone)]
pub(crate) struct UplevelInfo {
    pub idx_in_uplevel: u8,
}

#[derive(Copy, Clone)]
struct UplevelSymbolResolution {
    depth: u8,
    index_at_depth: u8,
}

pub struct FunctionRootScope {
    symbols: RefCell<HashMap<String, u8>>,
    index_provider: RefCell<IndexProviderImpl<u8>>,
    parent: CompilationScope,
    lexical_parent: Option<(CompilationScope, BasicBlock)>,
    pub(crate) uplevels: RefCell<Vec<UplevelInfo>>,
}

impl FunctionRootScope {
    fn root_function(parent: CompilationScope) -> Self {
        Self {
            symbols: Default::default(),
            index_provider: Default::default(),
            parent: parent.get_module_scope().unwrap(),
            lexical_parent: None,
            uplevels: Default::default(),
        }
    }

    fn closure(lexical_parent: (CompilationScope, BasicBlock)) -> Self {
        Self {
            symbols: Default::default(),
            index_provider: Default::default(),
            parent: lexical_parent.0.get_module_scope().unwrap(),
            lexical_parent: Some(lexical_parent),
            uplevels: Default::default(),
        }
    }

    pub fn num_locals(&self) -> u8 {
        self.index_provider.borrow().get_max_index()
    }

    pub fn emit_typed_define(
        &self,
        name: &str,
        _: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        let next_idx = self.index_provider.borrow_mut().next();
        self.symbols.borrow_mut().insert(name.to_owned(), next_idx);
        dest.write_opcode_and_source_info(CompilerOpcode::TypedefLocal(next_idx), loc);
        Ok(())
    }

    pub fn emit_write(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::WriteLocal(*existing_idx), loc);
            Ok(())
        } else if let Some(uplevel_info) =
            self.resolve_uplevel_symbol(name, dest.clone(), loc.clone(), false)?
        {
            dest.write_opcode_and_source_info(
                CompilerOpcode::WriteLocal(uplevel_info.index_at_depth),
                loc.clone(),
            );
            Ok(())
        } else {
            self.parent.emit_write(name, consts, dest, loc)
        }
    }

    fn store_uplevel_as_local(
        &self,
        name: &str,
        dest: BasicBlock,
        loc: SourcePointer,
        uplevel: UplevelSymbolResolution,
        want_dup_on_stack: bool,
    ) -> ScopeResult<UplevelSymbolResolution> {
        if uplevel.depth > 1 {
            return Err(ScopeError {
                loc,
                reason: ScopeErrorReason::OverlyDeepClosure,
            });
        }
        let index_in_local = self.index_provider.borrow_mut().next();
        self.symbols
            .borrow_mut()
            .insert(name.to_owned(), index_in_local);
        self.uplevels.borrow_mut().push(UplevelInfo {
            idx_in_uplevel: uplevel.index_at_depth,
        });
        dest.write_opcode_and_source_info(
            CompilerOpcode::ReadUplevel(uplevel.index_at_depth),
            loc.clone(),
        );
        if want_dup_on_stack {
            dest.write_opcode_and_source_info(CompilerOpcode::Dup, loc.clone());
        }
        dest.write_opcode_and_source_info(CompilerOpcode::WriteLocal(index_in_local), loc);
        Ok(UplevelSymbolResolution {
            depth: 0,
            index_at_depth: index_in_local,
        })
    }

    pub fn emit_read(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        let maybe_idx = self.symbols.borrow().get(name).cloned();
        if let Some(existing_idx) = maybe_idx {
            dest.write_opcode_and_source_info(CompilerOpcode::ReadLocal(existing_idx), loc);
            return Ok(());
        }

        if self
            .resolve_uplevel_symbol(name, dest.clone(), loc.clone(), true)?
            .is_some()
        {
            return Ok(());
        }

        self.parent.emit_read(name, consts, dest, loc)
    }

    fn resolve_uplevel_symbol(
        &self,
        name: &str,
        dest: BasicBlock,
        loc: SourcePointer,
        want_dup_on_stack: bool,
    ) -> ScopeResult<Option<UplevelSymbolResolution>> {
        let maybe_idx = self.symbols.borrow().get(name).cloned();
        if let Some(existing_idx) = maybe_idx {
            Ok(Some(UplevelSymbolResolution {
                depth: 0,
                index_at_depth: existing_idx,
            }))
        } else if let Some(cp) = &self.lexical_parent {
            let sr =
                cp.0.resolve_uplevel_symbol(name, cp.1.clone(), loc.clone(), want_dup_on_stack)?;
            if let Some(sr) = sr {
                let sr = self.store_uplevel_as_local(name, dest, loc, sr, want_dup_on_stack)?;
                Ok(Some(sr))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

pub struct FunctionChildScope {
    symbols: RefCell<HashMap<String, u8>>,
    parent: CompilationScope,
}

impl FunctionChildScope {
    fn new(parent: CompilationScope) -> Self {
        Self {
            symbols: Default::default(),
            parent,
        }
    }

    fn get_function_root(&self) -> Rc<FunctionRootScope> {
        match &self.parent {
            CompilationScope::FunctionRoot(r) => r.clone(),
            CompilationScope::FunctionChild(c) => c.get_function_root(),
            _ => panic!("function scope should end in a function, not a module"),
        }
    }

    pub fn emit_typed_define(
        &self,
        name: &str,
        _: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        let next_idx = self.get_function_root().index_provider.borrow_mut().next();
        self.symbols.borrow_mut().insert(name.to_owned(), next_idx);
        dest.write_opcode_and_source_info(CompilerOpcode::TypedefLocal(next_idx), loc);
        Ok(())
    }

    pub fn emit_write(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::WriteLocal(*existing_idx), loc);
            Ok(())
        } else {
            self.parent.emit_write(name, consts, dest, loc)
        }
    }

    pub fn emit_read(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            dest.write_opcode_and_source_info(CompilerOpcode::ReadLocal(*existing_idx), loc);
            Ok(())
        } else {
            self.parent.emit_read(name, consts, dest, loc)
        }
    }

    fn resolve_uplevel_symbol(
        &self,
        name: &str,
        dest: BasicBlock,
        loc: SourcePointer,
        want_dup_on_stack: bool,
    ) -> ScopeResult<Option<UplevelSymbolResolution>> {
        if let Some(existing_idx) = self.symbols.borrow().get(name) {
            Ok(Some(UplevelSymbolResolution {
                depth: 0,
                index_at_depth: *existing_idx,
            }))
        } else {
            self.parent
                .resolve_uplevel_symbol(name, dest, loc, want_dup_on_stack)
        }
    }
}

#[derive(enum_as_inner::EnumAsInner, Clone)]
pub enum CompilationScope {
    ModuleRoot(Rc<ModuleRootScope>),
    ModuleChild(Rc<ModuleChildScope>),
    FunctionRoot(Rc<FunctionRootScope>),
    FunctionChild(Rc<FunctionChildScope>),
}

impl CompilationScope {
    pub fn module() -> Self {
        Self::ModuleRoot(Rc::new(ModuleRootScope::default()))
    }

    pub fn function(&self) -> Self {
        Self::FunctionRoot(Rc::new(FunctionRootScope::root_function(self.clone())))
    }

    pub fn closure(&self, dest: BasicBlock) -> Self {
        Self::FunctionRoot(Rc::new(FunctionRootScope::closure((self.clone(), dest))))
    }

    pub(crate) fn get_module_scope(&self) -> Option<CompilationScope> {
        match self {
            Self::ModuleRoot(_) => Some(self.clone()),
            Self::ModuleChild(_) => Some(self.clone()),
            Self::FunctionRoot(fr) => fr.parent.get_module_scope(),
            Self::FunctionChild(fc) => fc.parent.get_module_scope(),
        }
    }

    pub fn child(&self) -> Self {
        match self {
            CompilationScope::ModuleRoot(_) | CompilationScope::ModuleChild(_) => {
                Self::ModuleChild(Rc::new(ModuleChildScope::new(self.clone())))
            }
            CompilationScope::FunctionRoot(_) | CompilationScope::FunctionChild(_) => {
                Self::FunctionChild(Rc::new(FunctionChildScope::new(self.clone())))
            }
        }
    }

    pub fn emit_typed_define(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        match self {
            Self::ModuleRoot(r) => r.emit_typed_define(name, consts, dest, loc),
            Self::FunctionRoot(r) => r.emit_typed_define(name, consts, dest, loc),
            Self::ModuleChild(c) => c.emit_typed_define(name, consts, dest, loc),
            Self::FunctionChild(c) => c.emit_typed_define(name, consts, dest, loc),
        }
    }

    pub fn emit_untyped_define(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        dest.write_opcode_and_source_info(
            CompilerOpcode::PushBuiltinTy(BUILTIN_TYPE_ANY),
            loc.clone(),
        );
        self.emit_typed_define(name, consts, dest.clone(), loc.clone())?;
        self.emit_write(name, consts, dest, loc)
    }

    pub fn emit_write(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        match self {
            Self::ModuleRoot(r) => r.emit_write(name, consts, dest, loc),
            Self::FunctionRoot(r) => r.emit_write(name, consts, dest, loc),
            Self::ModuleChild(c) => c.emit_write(name, consts, dest, loc),
            Self::FunctionChild(c) => c.emit_write(name, consts, dest, loc),
        }
    }

    pub fn emit_read(
        &self,
        name: &str,
        consts: &mut ConstantValues,
        dest: BasicBlock,
        loc: SourcePointer,
    ) -> ScopeResult {
        match self {
            Self::ModuleRoot(r) => r.emit_read(name, consts, dest, loc),
            Self::FunctionRoot(r) => r.emit_read(name, consts, dest, loc),
            Self::ModuleChild(c) => c.emit_read(name, consts, dest, loc),
            Self::FunctionChild(c) => c.emit_read(name, consts, dest, loc),
        }
    }

    fn resolve_uplevel_symbol(
        &self,
        name: &str,
        dest: BasicBlock,
        loc: SourcePointer,
        want_dup_on_stack: bool,
    ) -> ScopeResult<Option<UplevelSymbolResolution>> {
        match self {
            Self::ModuleRoot(r) => r.resolve_uplevel_symbol(name, dest, loc, want_dup_on_stack),
            Self::ModuleChild(c) => c.resolve_uplevel_symbol(name, dest, loc, want_dup_on_stack),
            Self::FunctionRoot(r) => r.resolve_uplevel_symbol(name, dest, loc, want_dup_on_stack),
            Self::FunctionChild(c) => c.resolve_uplevel_symbol(name, dest, loc, want_dup_on_stack),
        }
    }
}
