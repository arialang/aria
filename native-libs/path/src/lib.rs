// SPDX-License-Identifier: Apache-2.0

use haxby_opcodes::function_attribs::{FUNC_IS_METHOD, METHOD_ATTRIBUTE_TYPE};
use haxby_vm::{
    builtins::{
        VmGlobals,
        native_iterator::{AriaNativeIterator, NativeIteratorImpl, create_iterator_struct},
    },
    error::{dylib_load::LoadResult, vm_error::VmErrorReason},
    frame::Frame,
    runtime_module::RuntimeModule,
    runtime_value::{
        RuntimeValue, function::BuiltinFunctionImpl, object::Object, opaque::OpaqueValue,
    },
    symbol::Symbol,
    vm::{self, RunloopExit},
};

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::SystemTime};

struct MutablePath {
    content: RefCell<std::path::PathBuf>,
}

fn new_from_path<P: AsRef<std::path::Path>>(
    the_struct: &haxby_vm::runtime_value::structure::Struct,
    the_path: P,
    path_sym: Symbol,
    builtins: &mut VmGlobals,
) -> RuntimeValue {
    let pb = PathBuf::from(the_path.as_ref());
    let pb = MutablePath {
        content: RefCell::new(pb),
    };

    let path_obj = OpaqueValue::new(pb);
    RuntimeValue::Object(Object::new(the_struct).with_value(
        builtins,
        path_sym,
        RuntimeValue::Opaque(path_obj),
    ))
}

fn create_path_result_err(
    path_struct: &haxby_vm::runtime_value::structure::Struct,
    message: String,
    vm: &mut vm::VirtualMachine,
) -> Result<RuntimeValue, VmErrorReason> {
    let error_sym = vm
        .globals
        .intern_symbol("Error")
        .expect("too many symbols interned");
    let path_error = path_struct.extract_field(&vm.globals, error_sym, |field: RuntimeValue| {
        field.as_struct().cloned()
    })?;

    let path_error = RuntimeValue::Object(Object::new(&path_error));
    let msg_sym = vm
        .globals
        .intern_symbol("msg")
        .expect("too many symbols interned");
    let _ = path_error.write_attribute(
        msg_sym,
        RuntimeValue::String(message.into()),
        &mut vm.globals,
    );

    vm.globals.create_result_err(path_error)
}

fn mut_path_from_aria(
    aria_object: &Object,
    builtins: &VmGlobals,
) -> Result<Rc<MutablePath>, VmErrorReason> {
    let path_sym = builtins
        .lookup_symbol("__path")
        .ok_or(VmErrorReason::UnexpectedVmState)?;
    let rust_obj = aria_object
        .read(builtins, path_sym)
        .ok_or(VmErrorReason::UnexpectedVmState)?;
    rust_obj
        .as_opaque_concrete::<MutablePath>()
        .ok_or(VmErrorReason::UnexpectedVmState)
}

fn path_symbol(vm: &mut vm::VirtualMachine) -> Symbol {
    vm.globals
        .intern_symbol("__path")
        .expect("too many symbols interned")
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

        let path_sym = path_symbol(vm);
        frame.stack.push(new_from_path(
            &the_struct,
            the_path,
            path_sym,
            &mut vm.globals,
        ));
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

struct PathBufAriaIterator {
    iter: Box<dyn Iterator<Item = PathBuf>>,
    the_struct: haxby_vm::runtime_value::structure::Struct,
    path_sym: Symbol,
}

impl AriaNativeIterator for PathBufAriaIterator {
    type Item = RuntimeValue;

    fn next(&mut self, vm: &mut crate::vm::VirtualMachine) -> Option<Self::Item> {
        let next_pathbuf = self.iter.next()?;

        let next_runtime_val = new_from_path(
            &self.the_struct,
            next_pathbuf,
            self.path_sym,
            &mut vm.globals,
        );

        Some(next_runtime_val)
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
        let path_sym = path_symbol(vm);

        let val = match glob::glob(&glob_expr) {
            Ok(path) => {
                let iterator_sym = vm
                    .globals
                    .intern_symbol("Iterator")
                    .expect("too many symbols interned");
                let iterator_rv = the_struct
                    .load_named_value(&vm.globals, iterator_sym)
                    .ok_or(VmErrorReason::UnexpectedVmState)?;
                let iterator_struct = iterator_rv
                    .as_struct()
                    .ok_or(VmErrorReason::UnexpectedVmState)?;

                let values = path.flatten();

                let iterator = create_iterator_struct(
                    iterator_struct,
                    NativeIteratorImpl::new(PathBufAriaIterator {
                        iter: Box::new(values),
                        the_struct: the_struct.clone(),
                        path_sym,
                    }),
                    &mut vm.globals,
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

        let path_sym = path_symbol(vm);
        frame
            .stack
            .push(new_from_path(&the_struct, &cwd, path_sym, &mut vm.globals));
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
        let path_sym = path_symbol(vm);

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;

        let rfo = rust_obj.content.borrow_mut();
        let val = match rfo.canonicalize() {
            Ok(path) => {
                let canonical_object =
                    new_from_path(aria_object.get_struct(), &path, path_sym, &mut vm.globals);

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
        let path_sym = path_symbol(vm);

        let aria_struct = aria_object.get_struct().clone();
        let iterator_sym = vm
            .globals
            .intern_symbol("Iterator")
            .expect("too many symbols interned");
        let iterator_struct =
            aria_struct.extract_field(&vm.globals, iterator_sym, |f: RuntimeValue| {
                f.as_struct().cloned()
            })?;

        let rust_obj = mut_path_from_aria(&aria_object, &vm.globals)?;
        let rfo = rust_obj.content.borrow_mut();

        if let Ok(rd) = rfo.read_dir() {
            let values = rd.flatten().map(|e| e.path());

            let iterator = create_iterator_struct(
                &iterator_struct,
                NativeIteratorImpl::new(PathBufAriaIterator {
                    iter: Box::new(values),
                    the_struct: aria_struct.clone(),
                    path_sym,
                }),
                &mut vm.globals,
            );

            frame.stack.push(iterator);
        } else {
            let iterator = create_iterator_struct(
                &iterator_struct,
                NativeIteratorImpl::empty(),
                &mut vm.globals,
            );
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
        let path_sym = path_symbol(vm);

        let this_path = mut_path_from_aria(&this_path, &vm.globals)?;
        let other_path = mut_path_from_aria(&other_path, &vm.globals)?;

        let this_path = this_path.content.borrow_mut();
        let other_path = other_path.content.borrow_mut();

        let val = match this_path.ancestors().find(|p| other_path.starts_with(p)) {
            Some(p) => {
                let candidate = new_from_path(this_struct, p, path_sym, &mut vm.globals);
                vm.globals.create_maybe_some(candidate)?
            }
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
    match unsafe {
        (
            (vm as *mut haxby_vm::vm::VirtualMachine).as_mut(),
            module.as_ref(),
        )
    } {
        (Some(vm), Some(module)) => {
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

            path_struct.insert_builtin::<New>(&mut vm.globals);
            path_struct.insert_builtin::<Glob>(&mut vm.globals);
            path_struct.insert_builtin::<Cwd>(&mut vm.globals);
            path_struct.insert_builtin::<Prettyprint>(&mut vm.globals);
            path_struct.insert_builtin::<Append>(&mut vm.globals);
            path_struct.insert_builtin::<Pop>(&mut vm.globals);
            path_struct.insert_builtin::<IsAbsolutePath>(&mut vm.globals);
            path_struct.insert_builtin::<Exists>(&mut vm.globals);
            path_struct.insert_builtin::<IsDirectory>(&mut vm.globals);
            path_struct.insert_builtin::<IsSymlink>(&mut vm.globals);
            path_struct.insert_builtin::<IsFile>(&mut vm.globals);
            path_struct.insert_builtin::<Canonical>(&mut vm.globals);
            path_struct.insert_builtin::<Size>(&mut vm.globals);
            path_struct.insert_builtin::<Entries>(&mut vm.globals);
            path_struct.insert_builtin::<Filename>(&mut vm.globals);
            path_struct.insert_builtin::<Extension>(&mut vm.globals);
            path_struct.insert_builtin::<CreatedTime>(&mut vm.globals);
            path_struct.insert_builtin::<AccessedTime>(&mut vm.globals);
            path_struct.insert_builtin::<ModifiedTime>(&mut vm.globals);
            path_struct.insert_builtin::<MakeDirectories>(&mut vm.globals);
            path_struct.insert_builtin::<MakeDirectory>(&mut vm.globals);
            path_struct.insert_builtin::<RemoveDirectory>(&mut vm.globals);
            path_struct.insert_builtin::<RemoveFile>(&mut vm.globals);
            path_struct.insert_builtin::<Copy>(&mut vm.globals);
            path_struct.insert_builtin::<CommonAncestor>(&mut vm.globals);
            path_struct.insert_builtin::<Equals>(&mut vm.globals);

            LoadResult::success()
        }
        _ => LoadResult::error("invalid path module"),
    }
}
