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
    vm::{self, RunloopExit},
};

use std::{
    cell::RefCell,
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
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

fn throw_io_error(the_struct: &Struct, message: String) -> crate::vm::ExecutionResult<RunloopExit> {
    let io_error = the_struct.extract_field("IOError", |f| f.as_struct().cloned())?;
    let io_error = RuntimeValue::Object(Object::new(&io_error));
    let _ = io_error.write_attribute("message", RuntimeValue::String(message.into()));
    Ok(RunloopExit::Exception(VmException::from_value(io_error)))
}

#[derive(Default)]
struct New {}
impl BuiltinFunctionImpl for New {
    fn eval(
        &self,
        frame: &mut Frame,
        _: &mut crate::vm::VirtualMachine,
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
                let _ = aria_file_obj.write_attribute("__file", RuntimeValue::Opaque(file_obj));
                frame.stack.push(aria_file_obj);
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(&the_struct, format!("Failed to open file: {e}")),
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let _ = rust_file_obj.file.borrow_mut().flush();
        aria_file.delete("__file");
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let mut dest = String::new();
        {
            let mut file_ref = rust_file_obj.file.borrow_mut();
            match file_ref.read_to_string(&mut dest) {
                Ok(_) => {
                    frame.stack.push(RuntimeValue::String(dest.into()));
                    Ok(RunloopExit::Ok(()))
                }
                Err(e) => {
                    throw_io_error(aria_file.get_struct(), format!("Failed to read file: {e}"))
                }
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let count =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_integer().cloned())?.raw_value();

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

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
                Err(e) => {
                    throw_io_error(aria_file.get_struct(), format!("Failed to read file: {e}"))
                }
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let text =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let mut rfo = rust_file_obj.file.borrow_mut();
        match rfo.write(text.as_bytes()) {
            Ok(n) => {
                frame.stack.push(RuntimeValue::Integer((n as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(aria_file.get_struct(), format!("Failed to write file: {e}")),
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let mut rfo = rust_file_obj.file.borrow_mut();

        match rfo.stream_position() {
            Ok(n) => {
                frame.stack.push(RuntimeValue::Integer((n as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to get file position: {e}"),
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let offset =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_integer().cloned())?.raw_value();

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let mut rfo = rust_file_obj.file.borrow_mut();

        match rfo.seek(std::io::SeekFrom::Start(offset as u64)) {
            Ok(n) => {
                frame.stack.push(RuntimeValue::Integer((n as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(
                aria_file.get_struct(),
                format!("Failed to set file position: {e}"),
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let rfo = rust_file_obj.file.borrow_mut();

        match rfo.metadata() {
            Ok(m) => {
                frame
                    .stack
                    .push(RuntimeValue::Integer((m.len() as i64).into()));
                Ok(RunloopExit::Ok(()))
            }
            Err(e) => throw_io_error(aria_file.get_struct(), format!("Failed to flush file: {e}")),
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
        _: &mut crate::vm::VirtualMachine,
    ) -> crate::vm::ExecutionResult<RunloopExit> {
        let aria_file = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let rust_file_obj = match aria_file.read("__file") {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_file_obj = match rust_file_obj.as_opaque_concrete::<MutableFile>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let mut rfo = rust_file_obj.file.borrow_mut();

        match rfo.flush() {
            Ok(_) => Ok(RunloopExit::Ok(())),
            Err(e) => throw_io_error(aria_file.get_struct(), format!("Failed to flush file: {e}")),
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
    _: *const haxby_vm::vm::VirtualMachine,
    module: *const RuntimeModule,
) -> LoadResult {
    match unsafe { module.as_ref() } {
        Some(module) => {
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

            file_struct.insert_builtin::<New>();
            file_struct.insert_builtin::<Close>();
            file_struct.insert_builtin::<ReadAll>();
            file_struct.insert_builtin::<ReadCount>();
            file_struct.insert_builtin::<WriteStr>();
            file_struct.insert_builtin::<GetPos>();
            file_struct.insert_builtin::<SetPos>();
            file_struct.insert_builtin::<Flush>();
            file_struct.insert_builtin::<GetSize>();

            LoadResult::success()
        }
        None => LoadResult::error("invalid file module"),
    }
}
