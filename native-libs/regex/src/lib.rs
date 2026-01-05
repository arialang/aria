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

fn create_regex_error(
    regex_struct: &Struct,
    message: String,
) -> Result<RuntimeValue, VmErrorReason> {
    let regex_error = regex_struct
        .load_named_value("Error")
        .ok_or(VmErrorReason::UnexpectedVmState)?;

    let regex_error = regex_error
        .as_struct()
        .ok_or(VmErrorReason::UnexpectedType)?;

    let regex_error = RuntimeValue::Object(Object::new(regex_error));
    let _ = regex_error.write_attribute("msg", RuntimeValue::String(message.into()));

    Ok(regex_error)
}

#[derive(Default)]
struct New {}
impl BuiltinFunctionImpl for New {
    fn eval(&self, frame: &mut Frame, _: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let the_struct = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_struct().cloned())?;
        let the_pattern = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?;

        let rust_regex_obj = match regex::Regex::new(&the_pattern.raw_value()) {
            Ok(s) => s,
            Err(e) => {
                let err = create_regex_error(&the_struct, e.to_string());
                return match err {
                    Ok(s) => Ok(RunloopExit::Exception(VmException::from_value(s))),
                    Err(e) => Err(e.into()),
                };
            }
        };

        let rust_regex_obj = OpaqueValue::new(rust_regex_obj);

        let aria_regex_obj = RuntimeValue::Object(Object::new(&the_struct));
        let _ = aria_regex_obj.write_attribute("__pattern", RuntimeValue::Opaque(rust_regex_obj));
        let _ = aria_regex_obj.write_attribute("pattern", RuntimeValue::String(the_pattern));

        frame.stack.push(aria_regex_obj);
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
    fn eval(&self, frame: &mut Frame, _: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let aria_regex = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let the_haystack = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?;

        let rust_regex_obj = match aria_regex.read("__pattern") {
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
    fn eval(&self, frame: &mut Frame, _: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let aria_regex = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;
        let aria_struct = aria_regex.get_struct().clone();
        let the_haystack =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let match_struct_type = aria_struct.extract_field("Match", |e| e.as_struct().cloned())?;

        let rust_regex_obj = match aria_regex.read("__pattern") {
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

        let matches_list = List::default();
        for m in matches {
            let match_obj = RuntimeValue::Object(Object::new(&match_struct_type));
            let _ = match_obj.write_attribute("start", RuntimeValue::Integer(m.0.into()));
            let _ = match_obj.write_attribute("len", RuntimeValue::Integer(m.1.into()));
            let _ = match_obj.write_attribute("value", RuntimeValue::String(m.2.into()));
            matches_list.append(match_obj);
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
    fn eval(&self, frame: &mut Frame, _: &mut VirtualMachine) -> ExecutionResult<RunloopExit> {
        let aria_regex = VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_object().cloned())?;

        let the_haystack =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let new_value =
            VmGlobals::extract_arg(frame, |x: RuntimeValue| x.as_string().cloned())?.raw_value();

        let rust_regex_obj = match aria_regex.read("__pattern") {
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
    _: *const haxby_vm::vm::VirtualMachine,
    module: *const RuntimeModule,
) -> LoadResult {
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

            regex.insert_builtin::<New>();
            regex.insert_builtin::<AnyMatch>();
            regex.insert_builtin::<Matches>();
            regex.insert_builtin::<Replace>();

            LoadResult::success()
        }
        None => LoadResult::error("invalid regex module"),
    }
}
