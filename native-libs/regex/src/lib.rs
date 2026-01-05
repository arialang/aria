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
    vm::{ExecutionResult, RunloopExit, VirtualMachine},
};

fn intern_symbol(
    builtins: &VmGlobals,
    name: &str,
) -> Result<haxby_vm::symbol::Symbol, VmErrorReason> {
    builtins
        .intern_symbol(name)
        .map_err(|_| VmErrorReason::UnexpectedVmState)
}

fn create_regex_error(
    regex_struct: &Struct,
    message: String,
    builtins: &VmGlobals,
) -> Result<RuntimeValue, VmErrorReason> {
    let error_sym = intern_symbol(builtins, "Error")?;
    let regex_error = regex_struct
        .load_named_value(error_sym)
        .ok_or(VmErrorReason::UnexpectedVmState)?;

    let regex_error = regex_error
        .as_struct()
        .ok_or(VmErrorReason::UnexpectedType)?;

    let regex_error = Object::new(regex_error);
    let msg_sym = intern_symbol(builtins, "msg")?;
    regex_error.write(msg_sym, RuntimeValue::String(message.into()));

    Ok(RuntimeValue::Object(regex_error))
}

#[derive(Default)]
struct New {}
impl BuiltinFunctionImpl for New {
    fn eval(&self, frame: &mut Frame, vm: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let the_struct = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_struct().cloned())?;
        let the_pattern = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?;

        let rust_regex_obj = match regex::Regex::new(&the_pattern.raw_value()) {
            Ok(s) => s,
            Err(e) => {
                let err = create_regex_error(&the_struct, e.to_string(), &vm.globals);
                return match err {
                    Ok(s) => Ok(RunloopExit::Exception(VmException::from_value(s))),
                    Err(e) => Err(e.into()),
                };
            }
        };

        let rust_regex_obj = OpaqueValue::new(rust_regex_obj);

        let aria_regex_obj = Object::new(&the_struct);
        let pattern_sym = intern_symbol(&vm.globals, "__pattern")?;
        let pattern_value_sym = intern_symbol(&vm.globals, "pattern")?;
        aria_regex_obj.write(pattern_sym, RuntimeValue::Opaque(rust_regex_obj));
        aria_regex_obj.write(pattern_value_sym, RuntimeValue::String(the_pattern));

        frame.stack.push(RuntimeValue::Object(aria_regex_obj));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD | METHOD_ATTRIBUTE_TYPE
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "new"
    }
}

#[derive(Default)]
struct AnyMatch {}
impl BuiltinFunctionImpl for AnyMatch {
    fn eval(&self, frame: &mut Frame, vm: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let aria_regex = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let the_haystack = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?;

        let pattern_sym = intern_symbol(&vm.globals, "__pattern")?;
        let rust_regex_obj = match aria_regex.read(pattern_sym) {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_regex_obj = match rust_regex_obj.as_opaque_concrete::<regex::Regex>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let matches = rust_regex_obj.is_match(&the_haystack.raw_value());

        frame.stack.push(RuntimeValue::Boolean(matches.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "any_match"
    }
}

#[derive(Default)]
struct Matches {}
impl BuiltinFunctionImpl for Matches {
    fn eval(&self, frame: &mut Frame, vm: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let aria_regex = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let aria_struct = aria_regex.get_struct().clone();
        let the_haystack =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let match_sym = intern_symbol(&vm.globals, "Match")?;
        let match_struct_type =
            aria_struct.extract_field(match_sym, &vm.globals, |e| e.as_struct().cloned())?;

        let pattern_sym = intern_symbol(&vm.globals, "__pattern")?;
        let rust_regex_obj = match aria_regex.read(pattern_sym) {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_regex_obj = match rust_regex_obj.as_opaque_concrete::<regex::Regex>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let matches: Vec<_> = rust_regex_obj
            .find_iter(&the_haystack)
            .map(|mh| (mh.start() as i64, mh.len() as i64, mh.as_str()))
            .collect();

        let start_sym = intern_symbol(&vm.globals, "start")?;
        let len_sym = intern_symbol(&vm.globals, "len")?;
        let value_sym = intern_symbol(&vm.globals, "value")?;

        let matches_list = List::default();
        for m in matches {
            let match_obj = Object::new(&match_struct_type);
            match_obj.write(start_sym, RuntimeValue::Integer(m.0.into()));
            match_obj.write(len_sym, RuntimeValue::Integer(m.1.into()));
            match_obj.write(value_sym, RuntimeValue::String(m.2.into()));
            matches_list.append(RuntimeValue::Object(match_obj));
        }

        frame.stack.push(RuntimeValue::List(matches_list));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(2)
    }

    fn name(&self) -> &str {
        "matches"
    }
}

#[derive(Default)]
struct Replace {}
impl BuiltinFunctionImpl for Replace {
    fn eval(&self, frame: &mut Frame, vm: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let aria_regex = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let the_haystack =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let new_value =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let pattern_sym = intern_symbol(&vm.globals, "__pattern")?;
        let rust_regex_obj = match aria_regex.read(pattern_sym) {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };
        let rust_regex_obj = match rust_regex_obj.as_opaque_concrete::<regex::Regex>() {
            Some(s) => s,
            None => return Err(VmErrorReason::UnexpectedVmState.into()),
        };

        let target = rust_regex_obj
            .replace_all(&the_haystack, new_value)
            .to_string();

        frame.stack.push(RuntimeValue::String(target.into()));
        Ok(RunloopExit::Ok(()))
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(3)
    }

    fn name(&self) -> &str {
        "replace"
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
            let regex = match module.load_named_value("Regex") {
                Some(regex) => regex,
                None => {
                    return LoadResult::error("cannot find Regex");
                }
            };

            let regex = match regex.as_struct() {
                Some(regex) => regex,
                None => {
                    return LoadResult::error("Regex is not a struct");
                }
            };

            regex.insert_builtin::<New>(&vm.globals);
            regex.insert_builtin::<AnyMatch>(&vm.globals);
            regex.insert_builtin::<Matches>(&vm.globals);
            regex.insert_builtin::<Replace>(&vm.globals);

            LoadResult::success()
        }
        None => LoadResult::error("invalid regex module"),
    }
}
