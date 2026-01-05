// SPDX-License-Identifier: Apache-2.0

use haxby_opcodes::function_attribs::{FUNC_IS_METHOD, METHOD_ATTRIBUTE_TYPE};
use haxby_vm::{
    builtins::{
        VmGlobals,
        native_iterator::{NativeIteratorImpl, create_iterator_struct},
    },
    error::{dylib_load::LoadResult, vm_error::VmErrorReason},
    frame::Frame,
    runtime_module::RuntimeModule,
    runtime_value::{
        RuntimeValue, function::BuiltinFunctionImpl, object::Object, opaque::OpaqueValue,
    },
    vm::{self, RunloopExit},
};

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::SystemTime};

struct MutablePath {
    content: RefCell<std::path::PathBuf>,
}

fn intern_symbol(
    builtins: &VmGlobals,
    name: &str,
) -> Result<haxby_vm::symbol::Symbol, VmErrorReason> {
    builtins
        .intern_symbol(name)
        .map_err(|_| VmErrorReason::UnexpectedVmState)
}

fn path_symbol(builtins: &VmGlobals) -> Result<haxby_vm::symbol::Symbol, VmErrorReason> {
    intern_symbol(builtins, "__path")
}

fn new_from_path<P: AsRef<std::path::Path>>(
    the_struct: &haxby_vm::runtime_value::structure::Struct,
    the_path: P,
    path_sym: haxby_vm::symbol::Symbol,
) -> RuntimeValue {
    let pb = PathBuf::from(the_path.as_ref());
    let pb = MutablePath {
        content: RefCell::new(pb),
    };

    let path_obj = OpaqueValue::new(pb);
    let aria_obj = Object::new(the_struct);
    aria_obj.write(path_sym, RuntimeValue::Opaque(path_obj));
    RuntimeValue::Object(aria_obj)
}

fn create_path_result_err(
    path_struct: &haxby_vm::runtime_value::structure::Struct,
    message: String,
    vm: &mut vm::VirtualMachine,
) -> Result<RuntimeValue, VmErrorReason> {
    let err_sym = intern_symbol(&vm.globals, "Error")?;
    let path_error =
        path_struct.extract_field(err_sym, &vm.globals, |field| field.as_struct().cloned())?;

    let path_error = Object::new(&path_error);
    let msg_sym = intern_symbol(&vm.globals, "msg")?;
    path_error.write(msg_sym, RuntimeValue::String(message.into()));

    vm.globals
        .create_result_err(RuntimeValue::Object(path_error))
}

fn mut_path_from_aria(
    aria_object: &Object,
    builtins: &VmGlobals,
) -> Result<Rc<MutablePath>, VmErrorReason> {
    let path_sym = path_symbol(builtins)?;
    let rust_obj = aria_object
        .read(path_sym)
        .ok_or(VmErrorReason::UnexpectedVmState)?;
    rust_obj
        .as_opaque_concrete::<MutablePath>()
        .ok_or(VmErrorReason::UnexpectedVmState)
}

#[derive(Default)]
struct New {}
impl BuiltinFunctionImpl for New {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let the_struct = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_struct().cloned())?;
        let the_path =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let path_sym = path_symbol(&vm.globals)?;
        frame
            .stack
            .push(new_from_path(&the_struct, the_path, path_sym));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD | METHOD_ATTRIBUTE_TYPE
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_new"
    }
}

#[derive(Default)]
struct Glob {}
impl BuiltinFunctionImpl for Glob {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let the_struct = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_struct().cloned())?;
        let glob_expr =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let val = match glob::glob(&glob_expr) {
            Ok(path) => {
                let iterator_sym = intern_symbol(&vm.globals, "Iterator")?;
                let iterator_rv = the_struct
                    .load_named_value(iterator_sym)
                    .ok_or(VmErrorReason::UnexpectedVmState)?;
                let iterator_struct = iterator_rv
                    .as_struct()
                    .ok_or(VmErrorReason::UnexpectedVmState)?;

                let path_sym = path_symbol(&vm.globals)?;
                let flatten = path
                    .flatten()
                    .map(move |e| new_from_path(&the_struct, e, path_sym));

                let iterator = create_iterator_struct(
                    iterator_struct,
                    NativeIteratorImpl::new(flatten),
                    &vm.globals,
                );

                vm.globals.create_result_ok(iterator)?
            }
            Err(e) => create_path_result_err(&the_struct, e.to_string(), vm)?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD | METHOD_ATTRIBUTE_TYPE
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_glob"
    }
}

#[derive(Default)]
struct Cwd {}
impl BuiltinFunctionImpl for Cwd {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let the_struct = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_struct().cloned())?;

        let cwd = std::env::current_dir().map_err(|_| VmErrorReason::UnexpectedVmState)?;

        let path_sym = path_symbol(&vm.globals)?;
        frame.stack.push(new_from_path(&the_struct, &cwd, path_sym));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD | METHOD_ATTRIBUTE_TYPE
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_cwd"
    }
}

#[derive(Default)]
struct Prettyprint {}
impl BuiltinFunctionImpl for Prettyprint {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();

        match rfo.as_os_str().to_str() {
            Some(s) => {
                frame.stack.push(RuntimeValue::String(s.into()));
                Ok(RunloopExit::Ok(()))
            }
            None => Err(VmErrorReason::UnexpectedVmState.into()),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "prettyprint"
    }
}

#[derive(Default)]
struct Append {}
impl BuiltinFunctionImpl for Append {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let the_path =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let mut rfo = rust_obj.content.borrow_mut();
        rfo.push(the_path);

        frame.stack.push(vm.globals.create_unit_object()?);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_append"
    }
}

#[derive(Default)]
struct Pop {}
impl BuiltinFunctionImpl for Pop {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let mut rfo = rust_obj.content.borrow_mut();
        rfo.pop();
        frame.stack.push(RuntimeValue::Object(aria_object));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "pop"
    }
}

#[derive(Default)]
struct IsAbsolutePath {}
impl BuiltinFunctionImpl for IsAbsolutePath {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame
            .stack
            .push(RuntimeValue::Boolean((rfo.is_absolute()).into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_absolute"
    }
}

#[derive(Default)]
struct Exists {}
impl BuiltinFunctionImpl for Exists {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame
            .stack
            .push(RuntimeValue::Boolean((rfo.exists()).into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "exists"
    }
}

#[derive(Default)]
struct IsDirectory {}
impl BuiltinFunctionImpl for IsDirectory {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame
            .stack
            .push(RuntimeValue::Boolean((rfo.is_dir()).into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_directory"
    }
}

#[derive(Default)]
struct IsFile {}
impl BuiltinFunctionImpl for IsFile {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame
            .stack
            .push(RuntimeValue::Boolean((rfo.is_file()).into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_file"
    }
}

#[derive(Default)]
struct IsSymlink {}
impl BuiltinFunctionImpl for IsSymlink {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame
            .stack
            .push(RuntimeValue::Boolean((rfo.is_symlink()).into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "is_symlink"
    }
}

#[derive(Default)]
struct Canonical {}
impl BuiltinFunctionImpl for Canonical {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;
        let path_sym = path_symbol(&vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        let val = match rfo.canonicalize() {
            Ok(path) => {
                let canonical_object = new_from_path(aria_object.get_struct(), &path, path_sym);

                vm.globals.create_result_ok(canonical_object)?
            }
            Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "new_canonical"
    }
}

#[derive(Default)]
struct Size {}
impl BuiltinFunctionImpl for Size {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        let val = match rfo.metadata() {
            Ok(md) => vm
                .globals
                .create_result_ok(RuntimeValue::Integer((md.len() as i64).into()))?,
            Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "size"
    }
}

#[derive(Default)]
struct CreatedTime {}
impl BuiltinFunctionImpl for CreatedTime {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        let val = match rfo.metadata() {
            Ok(md) => match md.created() {
                Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
                Ok(val) => {
                    let val = val
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    vm.globals
                        .create_result_ok(RuntimeValue::Integer((val as i64).into()))?
                }
            },
            Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_when_created"
    }
}

#[derive(Default)]
struct AccessedTime {}
impl BuiltinFunctionImpl for AccessedTime {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        let val = match rfo.metadata() {
            Ok(md) => match md.accessed() {
                Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
                Ok(val) => {
                    let val = val
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    vm.globals
                        .create_result_ok(RuntimeValue::Integer((val as i64).into()))?
                }
            },
            Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_when_accessed"
    }
}

#[derive(Default)]
struct ModifiedTime {}
impl BuiltinFunctionImpl for ModifiedTime {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        let val = match rfo.metadata() {
            Ok(md) => match md.modified() {
                Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
                Ok(val) => {
                    let val = val
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap()
                        .as_millis();
                    vm.globals
                        .create_result_ok(RuntimeValue::Integer((val as i64).into()))?
                }
            },
            Err(e) => create_path_result_err(aria_object.get_struct(), e.to_string(), vm)?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_when_modified"
    }
}

#[derive(Default)]
struct Filename {}
impl BuiltinFunctionImpl for Filename {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        match rfo.file_name() {
            Some(name) => {
                let name = name.to_str().ok_or(VmErrorReason::UnexpectedVmState)?;
                let val = vm
                    .globals
                    .create_maybe_some(RuntimeValue::String(name.into()))?;
                frame.stack.push(val);
            }
            None => {
                let val = vm.globals.create_maybe_none()?;
                frame.stack.push(val);
            }
        }
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "get_filename"
    }
}

#[derive(Default)]
struct Extension {}
impl BuiltinFunctionImpl for Extension {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        match rfo.extension() {
            Some(name) => {
                let name = name.to_str().ok_or(VmErrorReason::UnexpectedVmState)?;
                let val = vm
                    .globals
                    .create_maybe_some(RuntimeValue::String(name.into()))?;
                frame.stack.push(val);
            }
            None => {
                let val = vm.globals.create_maybe_none()?;
                frame.stack.push(val);
            }
        }
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "get_extension"
    }
}

#[derive(Default)]
struct Entries {}
impl BuiltinFunctionImpl for Entries {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let aria_struct = aria_object.get_struct().clone();
        let iterator_sym = intern_symbol(&vm.globals, "Iterator")?;
        let iterator_struct =
            aria_struct.extract_field(iterator_sym, &vm.globals, |f| f.as_struct().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;
        let rfo = rust_obj.content.borrow_mut();
        let path_sym = path_symbol(&vm.globals)?;

        if let Ok(rd) = rfo.read_dir() {
            let flatten = rd
                .flatten()
                .map(move |e| new_from_path(&aria_struct, e.path(), path_sym));
            let iterator = create_iterator_struct(
                &iterator_struct,
                NativeIteratorImpl::new(flatten),
                &vm.globals,
            );
            frame.stack.push(iterator);
        } else {
            let iterator =
                create_iterator_struct(&iterator_struct, NativeIteratorImpl::empty(), &vm.globals);
            frame.stack.push(iterator);
        }

        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "entries"
    }
}

#[derive(Default)]
struct MakeDirectory {}
impl BuiltinFunctionImpl for MakeDirectory {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame.stack.push(RuntimeValue::Boolean(
            std::fs::create_dir(rfo.as_path()).is_ok().into(),
        ));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "mkdir"
    }
}

#[derive(Default)]
struct MakeDirectories {}
impl BuiltinFunctionImpl for MakeDirectories {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame.stack.push(RuntimeValue::Boolean(
            std::fs::create_dir_all(rfo.as_path()).is_ok().into(),
        ));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "mkdirs"
    }
}

#[derive(Default)]
struct RemoveDirectory {}
impl BuiltinFunctionImpl for RemoveDirectory {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame.stack.push(RuntimeValue::Boolean(
            std::fs::remove_dir(rfo.as_path()).is_ok().into(),
        ));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "rmdir"
    }
}

#[derive(Default)]
struct RemoveFile {}
impl BuiltinFunctionImpl for RemoveFile {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let aria_object = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        frame.stack.push(RuntimeValue::Boolean(
            std::fs::remove_file(rfo.as_path()).is_ok().into(),
        ));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "erase"
    }
}

#[derive(Default)]
struct Copy {}
impl BuiltinFunctionImpl for Copy {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let this_path = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let other_path = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let this_path = mut_path_from_aria(&this_path, &vm.globals)?;
        let other_path = mut_path_from_aria(&other_path, &vm.globals)?;

        let this_path = this_path.content.borrow_mut();
        let other_path = other_path.content.borrow_mut();

        frame.stack.push(RuntimeValue::Boolean(
            std::fs::copy(this_path.as_path(), other_path.as_path())
                .is_ok()
                .into(),
        ));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_copy"
    }
}

#[derive(Default)]
struct CommonAncestor {}
impl BuiltinFunctionImpl for CommonAncestor {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let this_path = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let this_struct = this_path.get_struct();
        let other_path = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let this_path = mut_path_from_aria(&this_path, &vm.globals)?;
        let other_path = mut_path_from_aria(&other_path, &vm.globals)?;

        let this_path = this_path.content.borrow_mut();
        let other_path = other_path.content.borrow_mut();

        let path_sym = path_symbol(&vm.globals)?;
        let val = match this_path.ancestors().find(|p| other_path.starts_with(p)) {
            Some(p) => vm
                .globals
                .create_maybe_some(new_from_path(this_struct, p, path_sym))?,
            None => vm.globals.create_maybe_none()?,
        };

        frame.stack.push(val);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "common_ancestor"
    }
}

#[derive(Default)]
struct Equals {}
impl BuiltinFunctionImpl for Equals {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut vm::VirtualMachine,
    ) -> vm::ExecutionResult<RunloopExit> {
        let this_path = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let other_path = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let this_path = mut_path_from_aria(&this_path, &vm.globals)?;
        let other_path = mut_path_from_aria(&other_path, &vm.globals)?;

        let this_path = this_path.content.borrow_mut();
        let other_path = other_path.content.borrow_mut();

        frame
            .stack
            .push(RuntimeValue::Boolean((*this_path == *other_path).into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_op_impl_equals"
    }
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn dylib_haxby_inject(
    vm: *const haxby_vm::vm::VirtualMachine,
    module: *const RuntimeModule,
) -> LoadResult {
    let vm = match unsafe { vm.as_ref() } {
        Some(vm) => vm,
        None => {
            return LoadResult::error("invalid vm");
        }
    };
    match unsafe { module.as_ref() } {
        Some(module) => {
            let path = match module.load_named_value("Path") {
                Some(path) => path,
                None => {
                    return LoadResult::error("cannot find Path");
                }
            };

            let path_struct = match path.as_struct() {
                Some(path) => path,
                None => {
                    return LoadResult::error("Path is not a struct");
                }
            };

            path_struct.insert_builtin::<New>(&vm.globals);
            path_struct.insert_builtin::<Glob>(&vm.globals);
            path_struct.insert_builtin::<Cwd>(&vm.globals);
            path_struct.insert_builtin::<Prettyprint>(&vm.globals);
            path_struct.insert_builtin::<Append>(&vm.globals);
            path_struct.insert_builtin::<Pop>(&vm.globals);
            path_struct.insert_builtin::<IsAbsolutePath>(&vm.globals);
            path_struct.insert_builtin::<Exists>(&vm.globals);
            path_struct.insert_builtin::<IsDirectory>(&vm.globals);
            path_struct.insert_builtin::<IsSymlink>(&vm.globals);
            path_struct.insert_builtin::<IsFile>(&vm.globals);
            path_struct.insert_builtin::<Canonical>(&vm.globals);
            path_struct.insert_builtin::<Size>(&vm.globals);
            path_struct.insert_builtin::<Entries>(&vm.globals);
            path_struct.insert_builtin::<Filename>(&vm.globals);
            path_struct.insert_builtin::<Extension>(&vm.globals);
            path_struct.insert_builtin::<CreatedTime>(&vm.globals);
            path_struct.insert_builtin::<AccessedTime>(&vm.globals);
            path_struct.insert_builtin::<ModifiedTime>(&vm.globals);
            path_struct.insert_builtin::<MakeDirectories>(&vm.globals);
            path_struct.insert_builtin::<MakeDirectory>(&vm.globals);
            path_struct.insert_builtin::<RemoveDirectory>(&vm.globals);
            path_struct.insert_builtin::<RemoveFile>(&vm.globals);
            path_struct.insert_builtin::<Copy>(&vm.globals);
            path_struct.insert_builtin::<CommonAncestor>(&vm.globals);
            path_struct.insert_builtin::<Equals>(&vm.globals);

            LoadResult::success()
        }
        None => LoadResult::error("invalid path module"),
    }
}
