// SPDX-License-Identifier: Apache-2.0
use std::rc::Rc;

use haxby_opcodes::BuiltinTypeId;

use crate::{
    error::vm_error::VmErrorReason,
    frame::Frame,
    runtime_value::{
        RuntimeValue,
        function::{BuiltinFunctionImpl, Function},
        kind::RuntimeValueType,
        object::ObjectBox,
    },
};

mod alloc;
mod arity;
mod boolean;
mod cmdline_args;
mod exit;
mod float;
mod getenv;
mod hasattr;
mod integer;
mod list;
mod listattrs;
mod maybe;
pub mod native_iterator;
mod now;
mod prettyprint;
mod print;
mod println;
mod readattr;
mod readln;
mod result;
mod runtime_error;
mod setenv;
mod sleep;
mod string;
mod system;
mod typ;
mod typeof_builtin;
mod unimplemented;
mod unit;
mod writeattr;

pub struct VmGlobals {
    values: Rc<ObjectBox>,
}

impl VmGlobals {
    pub fn insert_builtin<T>(&mut self)
    where
        T: 'static + Default + BuiltinFunctionImpl,
    {
        let t = T::default();
        let name = t.name().to_owned();
        self.insert(&name, RuntimeValue::Function(Function::builtin_from(t)));
    }

    pub fn extract_arg<T, U>(frame: &mut Frame, f: T) -> crate::vm::ExecutionResult<U>
    where
        T: FnOnce(RuntimeValue) -> Option<U>,
    {
        let val = match frame.stack.try_pop() {
            Some(v) => v,
            None => {
                return Err(VmErrorReason::EmptyStack.into());
            }
        };

        match f(val) {
            Some(v) => Ok(v),
            None => Err(VmErrorReason::UnexpectedType.into()),
        }
    }
}

impl Default for VmGlobals {
    fn default() -> Self {
        let mut this = Self {
            values: Default::default(),
        };

        this.insert("Any", RuntimeValue::Type(RuntimeValueType::Any)); // Most anything needs Any
        this.insert("Module", RuntimeValue::Type(RuntimeValueType::Module));

        unit::insert_unit_builtins(&mut this);
        unimplemented::insert_unimplemented_builtins(&mut this);
        maybe::insert_maybe_builtins(&mut this);
        result::insert_result_builtins(&mut this);
        integer::insert_integer_builtins(&mut this); // RuntimeError needs Integer
        string::insert_string_builtins(&mut this); // and String
        runtime_error::insert_runtime_error_builtins(&mut this);

        // from here on out, any order is fine

        alloc::insert_builtins(&mut this);
        arity::insert_builtins(&mut this);
        boolean::insert_boolean_builtins(&mut this);
        cmdline_args::insert_builtins(&mut this);
        exit::insert_builtins(&mut this);
        float::insert_float_builtins(&mut this);
        getenv::insert_builtins(&mut this);
        hasattr::insert_builtins(&mut this);
        list::insert_list_builtins(&mut this);
        listattrs::insert_builtins(&mut this);
        now::insert_builtins(&mut this);
        prettyprint::insert_builtins(&mut this);
        print::insert_builtins(&mut this);
        println::insert_builtins(&mut this);
        readattr::insert_builtins(&mut this);
        readln::insert_builtins(&mut this);
        setenv::insert_builtins(&mut this);
        sleep::insert_builtins(&mut this);
        system::insert_builtins(&mut this);
        typ::insert_type_builtins(&mut this);
        typeof_builtin::insert_builtins(&mut this);
        writeattr::insert_builtins(&mut this);

        this
    }
}

impl VmGlobals {
    pub fn load_named_value(&self, name: &str) -> Option<RuntimeValue> {
        self.values.read(name)
    }

    pub fn insert(&self, name: &str, val: RuntimeValue) {
        if self.values.contains(name) {
            panic!("duplicate builtin {name}");
        }

        self.values.write(name, val);
    }

    pub fn get_builtin_type_by_name(&self, name: &str) -> Option<RuntimeValueType> {
        if let Some(bv) = self.load_named_value(name) {
            bv.as_type().cloned()
        } else {
            None
        }
    }

    pub fn get_builtin_type_by_id(&self, bt_id: BuiltinTypeId) -> Option<RuntimeValueType> {
        self.get_builtin_type_by_name(bt_id.name())
    }
}

impl VmGlobals {
    pub fn create_maybe_some(&self, x: RuntimeValue) -> Result<RuntimeValue, VmErrorReason> {
        let rt_maybe = self
            .get_builtin_type_by_id(BuiltinTypeId::Maybe)
            .ok_or(VmErrorReason::UnexpectedVmState)?;
        let rt_maybe_enum = rt_maybe.as_enum().ok_or(VmErrorReason::UnexpectedType)?;

        let some_idx = rt_maybe_enum
            .get_idx_of_case("Some")
            .ok_or_else(|| VmErrorReason::NoSuchCase("Some".to_owned()))?;

        let rv = rt_maybe_enum
            .make_value(some_idx, Some(x))
            .ok_or(VmErrorReason::UnexpectedVmState)?;

        Ok(RuntimeValue::EnumValue(rv))
    }

    pub fn create_result_ok(&self, x: RuntimeValue) -> Result<RuntimeValue, VmErrorReason> {
        let rt_result = self
            .get_builtin_type_by_id(BuiltinTypeId::Result)
            .ok_or(VmErrorReason::UnexpectedVmState)?;
        let rt_result_enum = rt_result.as_enum().ok_or(VmErrorReason::UnexpectedType)?;

        let ok_idx = rt_result_enum
            .get_idx_of_case("Ok")
            .ok_or_else(|| VmErrorReason::NoSuchCase("Ok".to_owned()))?;

        let rv = rt_result_enum
            .make_value(ok_idx, Some(x))
            .ok_or(VmErrorReason::UnexpectedVmState)?;

        Ok(RuntimeValue::EnumValue(rv))
    }

    pub fn create_maybe_none(&self) -> Result<RuntimeValue, VmErrorReason> {
        let rt_maybe = self
            .get_builtin_type_by_id(BuiltinTypeId::Maybe)
            .ok_or(VmErrorReason::UnexpectedVmState)?;
        let rt_maybe_enum = rt_maybe.as_enum().ok_or(VmErrorReason::UnexpectedType)?;

        let none_idx = rt_maybe_enum
            .get_idx_of_case("None")
            .ok_or_else(|| VmErrorReason::NoSuchCase("None".to_owned()))?;

        let rv = rt_maybe_enum
            .make_value(none_idx, None)
            .ok_or(VmErrorReason::UnexpectedVmState)?;

        Ok(RuntimeValue::EnumValue(rv))
    }

    pub fn create_result_err(&self, x: RuntimeValue) -> Result<RuntimeValue, VmErrorReason> {
        let rt_result = self
            .get_builtin_type_by_id(BuiltinTypeId::Result)
            .ok_or(VmErrorReason::UnexpectedVmState)?;
        let rt_result_enum = rt_result.as_enum().ok_or(VmErrorReason::UnexpectedType)?;

        let err_idx = rt_result_enum
            .get_idx_of_case("Err")
            .ok_or_else(|| VmErrorReason::NoSuchCase("Err".to_owned()))?;

        let rv = rt_result_enum
            .make_value(err_idx, Some(x))
            .ok_or(VmErrorReason::UnexpectedVmState)?;

        Ok(RuntimeValue::EnumValue(rv))
    }

    pub fn create_unit_object(&self) -> Result<RuntimeValue, VmErrorReason> {
        let rt_unit = self
            .get_builtin_type_by_id(BuiltinTypeId::Unit)
            .ok_or(VmErrorReason::UnexpectedVmState)?;
        let rt_unit_enum = rt_unit.as_enum().ok_or(VmErrorReason::UnexpectedType)?;

        let unit_idx = rt_unit_enum
            .get_idx_of_case("unit")
            .ok_or_else(|| VmErrorReason::NoSuchCase("unit".to_owned()))?;

        let rv = rt_unit_enum
            .make_value(unit_idx, None)
            .ok_or(VmErrorReason::UnexpectedVmState)?;

        Ok(RuntimeValue::EnumValue(rv))
    }
}
