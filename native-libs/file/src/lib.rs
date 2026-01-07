// SPDX-License-Identifier: Apache-2.0

use haxby_opcodes::function_attribs::{FUNC_IS_METHOD, METHOD_ATTRIBUTE_TYPE};
use haxby_vm::{
    builtins::VmGlobals,
    error::{dylib_load::LoadResult, exception::VmException, vm_error::VmErrorReason},
    frame::Frame,
    runtime_module::RuntimeModule,
    runtime_value::{
        RuntimeValue, function::BuiltinFunctionImpl, list::List, object::Object,
        opaque::OpaqueValue, structure::Struct,
    },
    symbol::Symbol,
    vm::{self, RunloopExit},
};

use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
    rc::Rc,
};

const FILE_MODE_READ: i64 = 1;
const FILE_MODE_WRITE: i64 = 2;
const FILE_MODE_APPEND: i64 = 4;
const FILE_MODE_TRUNCATE: i64 = 8;
const FILE_MODE_NEED_NEW: i64 = 16;

fn open_options_from_int(n: i64) -> OpenOptions {
    let mut opts = OpenOptions::new();

    if (n & FILE_MODE_READ) != 0 {
        opts.read(true);
    }

    if (n & FILE_MODE_WRITE) != 0 {
        opts.write(true);
        if (n & FILE_MODE_NEED_NEW) != 0 {
            opts.create_new(true);
        } else {
            opts.create(true);
        }
        if (n & FILE_MODE_TRUNCATE) != 0 {
            opts.truncate(true);
        }
    }

    if (n & FILE_MODE_APPEND) != 0 {
        opts.append(true);
        if (n & FILE_MODE_NEED_NEW) != 0 {
            opts.create_new(true);
        } else {
            opts.create(true);
        }
        if (n & FILE_MODE_TRUNCATE) != 0 {
            opts.truncate(true);
        }
    }

    opts
}

struct MutableFile {
    file: RefCell<File>,
}

fn file_symbol(builtins: &VmGlobals) -> Result<Symbol, VmErrorReason> {
    builtins
        .lookup_symbol("__file")
        .ok_or(VmErrorReason::UnexpectedVmState)
}

fn mut_file_from_aria(
    aria_file: &Object,
    builtins: &VmGlobals,
) -> Result<Rc<MutableFile>, VmErrorReason> {
    let file_sym = file_symbol(builtins)?;
    let rust_file_obj = aria_file
        .read(file_sym)
        .ok_or(VmErrorReason::UnexpectedVmState)?;
    rust_file_obj
        .as_opaque_concrete::<MutableFile>()
        .ok_or(VmErrorReason::UnexpectedVmState)
}

fn throw_io_error(
    the_struct: &Struct,
    message: String,
    builtins: &mut VmGlobals,
) -> crate::vm::ExecutionResult<RunloopExit> {
    let err_sym = builtins
        .intern_symbol("IOError")
        .expect("too many symbols interned");
    let io_error = the_struct.extract_field(err_sym, |f| f.as_struct().cloned())?;
    let io_error = RuntimeValue::Object(Object::new(&io_error));
    let message_sym = builtins
        .intern_symbol("message")
        .expect("too many symbols interned");
    let _ = io_error.write_attribute(message_sym, RuntimeValue::String(message.into()), builtins);
    Ok(RunloopExit::Exception(VmException::from_value(io_error)))
}

#[derive(Default)]
struct New {}
impl BuiltinFunctionImpl for New {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let the_struct = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_struct().cloned())?;
        let the_path =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();
        let the_mode =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_integer().cloned())?.raw_value();

        let opts = open_options_from_int(the_mode);
        match opts.open(the_path) {
            Ok(file) => {
                let file = MutableFile {
                    file: RefCell::new(file),
                };
                let file_obj = OpaqueValue::new(file);
                let aria_file_obj = RuntimeValue::Object(Object::new(&the_struct));
                let file_sym = vm
                    .globals
                    .intern_symbol("__file")
                    .expect("too many symbols interned");
                let _ = aria_file_obj.write_attribute(
                    file_sym,
                    RuntimeValue::Opaque(file_obj),
                    &vm.globals,
                );
                frame.stack.push(aria_file_obj);
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                &the_struct,
                format!("Failed to open file: {e}"),
                &mut vm.globals,
            ),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD | METHOD_ATTRIBUTE_TYPE
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(3)
    }

    fn name(&self) -> &str {
        "_new"
    }
}

#[derive(Default)]
struct Close {}
impl BuiltinFunctionImpl for Close {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;
        let _ = rust_file_obj.file.borrow_mut().flush();
        aria_file.delete(file_symbol(&vm.globals)?);
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::zero()
    }

    fn name(&self) -> &str {
        "_close"
    }
}

#[derive(Default)]
struct ReadAll {}
impl BuiltinFunctionImpl for ReadAll {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;
        let mut dest = String::new();
        {
            let mut file_ref = rust_file_obj.file.borrow_mut();
            match file_ref.read_to_string(&mut dest) {
                Ok(_) => {
                    frame.stack.push(RuntimeValue::String(dest.into()));
                    Ok(RunloopExit::Ok(()))
                }
                Err(e) => throw_io_error(
                    aria_file.get_struct(),
                    format!("Failed to read file: {e}"),
                    &mut vm.globals,
                ),
            }
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_read_all"
    }
}

#[derive(Default)]
struct ReadCount {}
impl BuiltinFunctionImpl for ReadCount {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let count =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_integer().cloned())?.raw_value();

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;

        let mut bytes = vec![0u8; count as usize];
        {
            let mut file_ref = rust_file_obj.file.borrow_mut();
            match file_ref.read_exact(&mut bytes) {
                Ok(_) => {
                    let result = bytes
                        .iter()
                        .map(|&b| b as i64)
                        .map(|n| RuntimeValue::Integer(n.into()))
                        .collect::<Vec<_>>();
                    let result = List::from(&result);

                    frame.stack.push(RuntimeValue::List(result));
                    Ok(RunloopExit::Ok(()))
                }
                Err(e) => throw_io_error(
                    aria_file.get_struct(),
                    format!("Failed to read file: {e}"),
                    &mut vm.globals,
                ),
            }
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_read_count"
    }
}

#[derive(Default)]
struct WriteStr {}
impl BuiltinFunctionImpl for WriteStr {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let text =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;

        let mut rfo = rust_file_obj.file.borrow_mut();
        match rfo.write(text.as_bytes()) {
            Ok(n) => {
                frame.stack.push(RuntimeValue::Integer((n as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to write file: {e}"),
                &mut vm.globals,
            ),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_write_str"
    }
}

#[derive(Default)]
struct GetPos {}
impl BuiltinFunctionImpl for GetPos {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;

        let mut rfo = rust_file_obj.file.borrow_mut();

        match rfo.stream_position() {
            Ok(n) => {
                frame.stack.push(RuntimeValue::Integer((n as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to get file position: {e}"),
                &mut vm.globals,
            ),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_getpos"
    }
}

#[derive(Default)]
struct SetPos {}
impl BuiltinFunctionImpl for SetPos {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let offset =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_integer().cloned())?.raw_value();

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;

        let mut rfo = rust_file_obj.file.borrow_mut();

        match rfo.seek(std::io::SeekFrom::Start(offset as u64)) {
            Ok(n) => {
                frame.stack.push(RuntimeValue::Integer((n as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to set file position: {e}"),
                &mut vm.globals,
            ),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "_setpos"
    }
}

#[derive(Default)]
struct GetSize {}
impl BuiltinFunctionImpl for GetSize {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;

        let rfo = rust_file_obj.file.borrow_mut();

        match rfo.metadata() {
            Ok(m) => {
                frame
                    .stack
                    .push(RuntimeValue::Integer((m.len() as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to flush file: {e}"),
                &mut vm.globals,
            ),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_getsize"
    }
}

#[derive(Default)]
struct Flush {}
impl BuiltinFunctionImpl for Flush {
    fn eval(
        &self,
        frame: &mut Frame,
        vm: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = mut_file_from_aria(&aria_file, &vm.globals)?;

        let mut rfo = rust_file_obj.file.borrow_mut();

        match rfo.flush() {
            Ok(_) => Ok(RunloopExit::Ok(())),
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to flush file: {e}"),
                &mut vm.globals,
            ),
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(1)
    }

    fn name(&self) -> &str {
        "_getsize"
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
            let file = match module.load_named_value("File") {
                Some(file) => file,
                None => {
                    return LoadResult::error("cannot find File");
                }
            };

            let file_struct = match file.as_struct() {
                Some(file) => file,
                None => {
                    return LoadResult::error("File is not a struct");
                }
            };

            file_struct.insert_builtin::<New>(&mut vm.globals);
            file_struct.insert_builtin::<Close>(&mut vm.globals);
            file_struct.insert_builtin::<ReadAll>(&mut vm.globals);
            file_struct.insert_builtin::<ReadCount>(&mut vm.globals);
            file_struct.insert_builtin::<WriteStr>(&mut vm.globals);
            file_struct.insert_builtin::<GetPos>(&mut vm.globals);
            file_struct.insert_builtin::<SetPos>(&mut vm.globals);
            file_struct.insert_builtin::<Flush>(&mut vm.globals);
            file_struct.insert_builtin::<GetSize>(&mut vm.globals);

            LoadResult::success()
        }
        _ => LoadResult::error("invalid file module"),
    }
}
