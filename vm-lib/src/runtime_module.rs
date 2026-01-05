// SPDX-License-Identifier: Apache-2.0
use std::{cell::RefCell, collections::HashSet, rc::Rc};

use aria_compiler::{
    bc_reader::{BytecodeReader, DecodeResult},
    module::CompiledModule,
};
use haxby_opcodes::Opcode;
use rustc_data_structures::fx::FxHashMap;

use crate::{
    builtins::VmGlobals,
    error::vm_error::VmErrorReason,
    runtime_value::{
        RuntimeValue,
        function::{BuiltinFunctionImpl, Function},
        isa::IsaCheckable,
    },
    vm::VirtualMachine,
};

#[derive(Clone)]
pub struct NamedValue {
    pub val: RuntimeValue,
    pub ty: IsaCheckable,
}

struct RuntimeModuleImpl {
    compiled_module: CompiledModule,
    indexed_constants: Vec<RuntimeValue>,
    values: RefCell<FxHashMap<String, NamedValue>>,
    entry_co: crate::runtime_value::runtime_code_object::CodeObject,
}

fn byte_array_to_opcode_array(bytes: &[u8]) -> DecodeResult<Vec<Opcode>> {
    let mut opcodes = Vec::new();
    let mut decoder = BytecodeReader::from(bytes);

    loop {
        let next = decoder.read_opcode();
        match next {
            Ok(op) => opcodes.push(op),
            Err(err) => {
                return match err {
                    aria_compiler::bc_reader::DecodeError::EndOfStream => Ok(opcodes),
                    _ => Err(err),
                };
            }
        }
    }
}

fn replace_attribute_access_with_interned(
    vm: &mut VirtualMachine,
    cm: &CompiledModule,
    opcodes: &mut Vec<Opcode>,
) -> Result<(), VmErrorReason> {
    for opcode in opcodes {
        if let Opcode::ReadAttribute(x) = opcode {
            let x_str = cm
                .load_indexed_const(*x)
                .expect("missing constant")
                .as_string()
                .expect("expected string constant")
                .clone();
            let sym = match vm.globals.intern_symbol(&x_str) {
                Ok(s) => s,
                Err(_) => return Err(VmErrorReason::UnexpectedVmState),
            };
            *opcode = Opcode::ReadAttributeInterned(sym.0);
        } else if let Opcode::WriteAttribute(x) = opcode {
            let x_str = cm
                .load_indexed_const(*x)
                .expect("missing constant")
                .as_string()
                .expect("expected string constant")
                .clone();
            let sym = match vm.globals.intern_symbol(&x_str) {
                Ok(s) => s,
                Err(_) => return Err(VmErrorReason::UnexpectedVmState),
            };
            *opcode = Opcode::WriteAttributeInterned(sym.0);
        } else if let Opcode::BindMethod(a, x) = opcode {
            let x_str = cm
                .load_indexed_const(*x)
                .expect("missing constant")
                .as_string()
                .expect("expected string constant")
                .clone();
            let sym = match vm.globals.intern_symbol(&x_str) {
                Ok(s) => s,
                Err(_) => return Err(VmErrorReason::UnexpectedVmState),
            };
            *opcode = Opcode::BindMethodInterned(*a, sym.0);
        } else if let Opcode::BindCase(a, x) = opcode {
            let x_str = cm
                .load_indexed_const(*x)
                .expect("missing constant")
                .as_string()
                .expect("expected string constant")
                .clone();
            let sym = match vm.globals.intern_symbol(&x_str) {
                Ok(s) => s,
                Err(_) => return Err(VmErrorReason::UnexpectedVmState),
            };
            *opcode = Opcode::BindCaseInterned(*a, sym.0);
        }
    }

    Ok(())
}

fn compiled_code_object_to_runtime_code_object(
    vm: &mut VirtualMachine,
    cm: &CompiledModule,
    cco: aria_compiler::constant_value::CompiledCodeObject,
) -> Result<crate::runtime_value::runtime_code_object::CodeObject, VmErrorReason> {
    let mut ops = byte_array_to_opcode_array(cco.body.as_slice())?;
    replace_attribute_access_with_interned(vm, cm, &mut ops)?;
    let body: Rc<[Opcode]> = ops.into();

    Ok(crate::runtime_value::runtime_code_object::CodeObject {
        name: cco.name.clone(),
        body,
        required_argc: cco.required_argc,
        default_argc: cco.default_argc,
        frame_size: cco.frame_size,
        loc: cco.loc.clone(),
        line_table: Rc::from(cco.line_table.clone()),
    })
}

fn compiled_constant_to_runtime_value(
    vm: &mut VirtualMachine,
    cm: &CompiledModule,
    value: aria_compiler::constant_value::ConstantValue,
) -> Result<RuntimeValue, VmErrorReason> {
    use aria_compiler::constant_value::ConstantValue::{
        CompiledCodeObject, Float, Integer, String,
    };
    match value {
        Integer(n) => Ok(RuntimeValue::Integer(From::from(n))),
        String(s) => Ok(RuntimeValue::String(s.into())),
        CompiledCodeObject(cco) => Ok(RuntimeValue::CodeObject(
            compiled_code_object_to_runtime_code_object(vm, cm, cco)?,
        )),
        Float(f) => Ok(RuntimeValue::Float(f.raw_value().into())),
    }
}

impl RuntimeModuleImpl {
    fn new(vm: &mut VirtualMachine, cm: CompiledModule) -> Result<Self, VmErrorReason> {
        let entry_co =
            compiled_code_object_to_runtime_code_object(vm, &cm, cm.load_entry_code_object())?;
        let mut this = Self {
            compiled_module: cm,
            indexed_constants: Vec::new(),
            values: Default::default(),
            entry_co,
        };

        let mut i = 0;
        while i < this.compiled_module.constants.len() {
            let c = this
                .compiled_module
                .load_indexed_const(i as u16)
                .expect("module has missing constant data");

            let r = compiled_constant_to_runtime_value(vm, &this.compiled_module, c)?;
            this.indexed_constants.push(r);

            i += 1;
        }

        Ok(this)
    }

    fn named_values_of_this(&self) -> Vec<(String, NamedValue)> {
        let mut ret = vec![];

        for (n, v) in self.values.borrow().iter() {
            ret.push((n.clone(), v.clone()));
        }

        ret
    }

    fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        self.values.borrow().get(name).map(|v| v.val.clone())
    }

    fn typedef_named_value(&self, name: &str, ty: IsaCheckable) {
        let mut bm = self.values.borrow_mut();
        if let Some(val) = bm.get_mut(name) {
            val.ty = ty;
        } else {
            bm.insert(
                name.to_owned(),
                NamedValue {
                    val: RuntimeValue::Integer(0.into()),
                    ty,
                },
            );
        }
    }

    fn store_typechecked_named_value(
        &self,
        name: &str,
        val: RuntimeValue,
        builtins: &VmGlobals,
    ) -> Result<(), VmErrorReason> {
        let mut bm = self.values.borrow_mut();
        if let Some(nval) = bm.get_mut(name) {
            if !nval.ty.isa_check(&val, builtins) {
                Err(VmErrorReason::UnexpectedType)
            } else {
                nval.val = val;
                Ok(())
            }
        } else {
            Err(VmErrorReason::NoSuchIdentifier(name.to_owned()))
        }
    }

    fn store_named_value(&self, name: &str, val: RuntimeValue) {
        let mut bm = self.values.borrow_mut();
        if let Some(nval) = bm.get_mut(name) {
            nval.val = val;
        } else {
            bm.insert(
                name.to_owned(),
                NamedValue {
                    val,
                    ty: IsaCheckable::any(),
                },
            );
        }
    }

    fn load_indexed_const(&self, idx: u16) -> Option<&RuntimeValue> {
        self.indexed_constants.get(idx as usize)
    }

    fn list_named_values(&self) -> HashSet<String> {
        self.values.borrow().keys().cloned().collect()
    }
}

#[derive(Clone)]
pub struct RuntimeModule {
    imp: Rc<RuntimeModuleImpl>,
}

impl RuntimeModule {
    pub fn new(vm: &mut VirtualMachine, cm: CompiledModule) -> Result<Self, VmErrorReason> {
        Ok(Self {
            imp: Rc::new(RuntimeModuleImpl::new(vm, cm)?),
        })
    }

    pub(crate) fn named_values_of_this(&self) -> Vec<(String, NamedValue)> {
        self.imp.named_values_of_this()
    }

    pub(crate) fn get_compiled_module(&self) -> &CompiledModule {
        &self.imp.compiled_module
    }

    pub fn load_entry_code_object(&self) -> &crate::runtime_value::runtime_code_object::CodeObject {
        &self.imp.entry_co
    }

    pub fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        self.imp.load_named_value(name)
    }

    pub fn typedef_named_value(&self, name: &str, ty: IsaCheckable) {
        self.imp.typedef_named_value(name, ty)
    }

    pub fn store_named_value(&self, name: &str, val: RuntimeValue) {
        self.imp.store_named_value(name, val)
    }

    pub fn list_named_values(&self) -> HashSet<String> {
        self.imp.list_named_values()
    }

    pub fn store_typechecked_named_value(
        &self,
        name: &str,
        val: RuntimeValue,
        builtins: &VmGlobals,
    ) -> Result<(), VmErrorReason> {
        self.imp.store_typechecked_named_value(name, val, builtins)
    }

    pub fn load_indexed_const(&self, idx: u16) -> Option<&RuntimeValue> {
        self.imp.load_indexed_const(idx)
    }

    pub fn lift_all_symbols_from_other(
        &self,
        prior_art: &Self,
        vm: &crate::VirtualMachine,
    ) -> Result<(), VmErrorReason> {
        for (name, val) in prior_art.named_values_of_this() {
            self.typedef_named_value(&name, val.ty.clone());
            self.store_typechecked_named_value(&name, val.val.clone(), &vm.globals)?;
        }
        Ok(())
    }

    pub fn extract_value<T, U>(&self, name: &str, f: T) -> Option<U>
    where
        T: FnOnce(RuntimeValue) -> Option<U>,
    {
        f(self.load_named_value(name)?)
    }

    pub fn insert_builtin<T>(&self)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name = t.name().to_owned();
        self.store_named_value(&name, RuntimeValue::Function(Function::builtin_from(t)));
    }
}

impl PartialEq for RuntimeModule {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.imp, &other.imp)
    }
}
impl Eq for RuntimeModule {}
