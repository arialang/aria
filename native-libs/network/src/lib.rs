// SPDX-License-Identifier: Apache-2.0
use haxby_opcodes::function_attribs::FUNC_IS_METHOD;
use haxby_vm::{
    error::dylib_load::LoadResult,
    runtime_module::RuntimeModule,
    runtime_value::{RuntimeValue, list::List, object::Object},
    vm::ExecutionResult,
};

#[derive(Default)]
struct RequestGet {}
impl haxby_vm::runtime_value::function::BuiltinFunctionImpl for RequestGet {
    fn eval(
        &self,
        frame: &mut haxby_vm::frame::Frame,
        vm: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<haxby_vm::vm::RunloopExit> {
        let this = haxby_vm::builtins::VmBuiltins::extract_arg(frame, |x| x.as_object().cloned())?;
        let headers = haxby_vm::builtins::VmBuiltins::extract_arg(frame, |x| x.as_list().cloned())?;
        let this_url = this.extract_field("url", |field| field.as_string().cloned())?;
        let this_timeout = this.extract_field("timeout", |field| field.as_float().cloned())?;
        let as_struct = this.get_struct();
        let this_response =
            as_struct.extract_field("Response", |field| field.as_struct().cloned())?;
        let this_error = as_struct.extract_field("Error", |field| field.as_struct().cloned())?;

        let mut client = reqwest::blocking::Client::new()
            .get(this_url.raw_value())
            .timeout(std::time::Duration::from_secs_f64(this_timeout.raw_value()));
        for i in 0..headers.len() {
            let header = headers.get_at(i).unwrap();
            if let Some(list) = header.as_list()
                && list.len() == 2
            {
                let key = list.get_at(0).unwrap();
                let value = list.get_at(1).unwrap();
                if let (Some(key), Some(value)) = (key.as_string(), value.as_string()) {
                    client = client.header(key.raw_value(), value.raw_value());
                }
            }
        }

        match client.send() {
            Ok(r) => {
                let response_obj = Object::new(&this_response);
                response_obj.write(
                    "status_code",
                    haxby_vm::runtime_value::RuntimeValue::Integer(
                        (r.status().as_u16() as i64).into(),
                    ),
                );
                let header_list = List::from(&[]);
                for header in r.headers() {
                    let header_kvp = List::from(&[
                        RuntimeValue::String(header.0.as_str().into()),
                        RuntimeValue::String(header.1.to_str().unwrap_or("<err>").into()),
                    ]);
                    header_list.append(RuntimeValue::List(header_kvp));
                }
                response_obj.write("headers", RuntimeValue::List(header_list));
                match r.text() {
                    Ok(content) => {
                        response_obj.write("content", RuntimeValue::String(content.into()));
                    }
                    _ => {
                        let error_obj = Object::new(&this_error);
                        error_obj.write(
                            "msg",
                            RuntimeValue::String("content is not a valid String".into()),
                        );
                        let result_err = vm
                            .builtins
                            .create_result_err(RuntimeValue::Object(error_obj))?;
                        frame.stack.push(result_err);
                        return ExecutionResult::Ok(haxby_vm::vm::RunloopExit::Ok(()));
                    }
                }

                let result_ok = vm
                    .builtins
                    .create_result_ok(RuntimeValue::Object(response_obj.clone()))?;

                frame.stack.push(result_ok);
                Ok(haxby_vm::vm::RunloopExit::Ok(()))
            }
            Err(e) => {
                let error_obj = Object::new(&this_error);
                error_obj.write("msg", RuntimeValue::String(e.to_string().into()));
                let result_err = vm
                    .builtins
                    .create_result_err(RuntimeValue::Object(error_obj))?;

                frame.stack.push(result_err);
                ExecutionResult::Ok(haxby_vm::vm::RunloopExit::Ok(()))
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
        "_get"
    }
}

#[derive(Default)]
struct RequestPost {}
impl haxby_vm::runtime_value::function::BuiltinFunctionImpl for RequestPost {
    fn eval(
        &self,
        frame: &mut haxby_vm::frame::Frame,
        vm: &mut haxby_vm::vm::VirtualMachine,
    ) -> haxby_vm::vm::ExecutionResult<haxby_vm::vm::RunloopExit> {
        let this = haxby_vm::builtins::VmBuiltins::extract_arg(frame, |x| x.as_object().cloned())?;
        let headers = haxby_vm::builtins::VmBuiltins::extract_arg(frame, |x| x.as_list().cloned())?;
        let payload =
            haxby_vm::builtins::VmBuiltins::extract_arg(frame, |x| x.as_string().cloned())?;

        let this_url = this.extract_field("url", |field| field.as_string().cloned())?;
        let this_timeout = this.extract_field("timeout", |field| field.as_float().cloned())?;
        let as_struct = this.get_struct();
        let this_response =
            as_struct.extract_field("Response", |field| field.as_struct().cloned())?;
        let this_error = as_struct.extract_field("Error", |field| field.as_struct().cloned())?;

        let mut client = reqwest::blocking::Client::new()
            .post(this_url.raw_value())
            .body(payload.raw_value())
            .timeout(std::time::Duration::from_secs_f64(this_timeout.raw_value()));
        for i in 0..headers.len() {
            let header = headers.get_at(i).unwrap();
            if let Some(list) = header.as_list()
                && list.len() == 2
            {
                let key = list.get_at(0).unwrap();
                let value = list.get_at(1).unwrap();
                if let (Some(key), Some(value)) = (key.as_string(), value.as_string()) {
                    client = client.header(key.raw_value(), value.raw_value());
                }
            }
        }

        match client.send() {
            Ok(r) => {
                let response_obj = Object::new(&this_response);
                response_obj.write(
                    "status_code",
                    haxby_vm::runtime_value::RuntimeValue::Integer(
                        (r.status().as_u16() as i64).into(),
                    ),
                );
                let header_list = List::from(&[]);
                for header in r.headers() {
                    let header_kvp = List::from(&[
                        RuntimeValue::String(header.0.as_str().into()),
                        RuntimeValue::String(header.1.to_str().unwrap_or("<err>").into()),
                    ]);
                    header_list.append(RuntimeValue::List(header_kvp));
                }
                response_obj.write("headers", RuntimeValue::List(header_list));
                match r.text() {
                    Ok(content) => {
                        response_obj.write("content", RuntimeValue::String(content.into()));
                    }
                    _ => {
                        let error_obj = Object::new(&this_error);
                        error_obj.write(
                            "msg",
                            RuntimeValue::String("content is not a valid String".into()),
                        );
                        let result_err = vm
                            .builtins
                            .create_result_err(RuntimeValue::Object(error_obj))?;

                        frame.stack.push(result_err);
                        return ExecutionResult::Ok(haxby_vm::vm::RunloopExit::Ok(()));
                    }
                }

                let result_ok = vm
                    .builtins
                    .create_result_ok(RuntimeValue::Object(response_obj.clone()))?;

                frame.stack.push(result_ok);
                Ok(haxby_vm::vm::RunloopExit::Ok(()))
            }
            Err(e) => {
                let error_obj = Object::new(&this_error);
                error_obj.write("msg", RuntimeValue::String(e.to_string().into()));
                let result_err = vm
                    .builtins
                    .create_result_err(RuntimeValue::Object(error_obj))?;

                frame.stack.push(result_err);
                ExecutionResult::Ok(haxby_vm::vm::RunloopExit::Ok(()))
            }
        }
    }

    fn attrib_byte(&self) -> u8 {
        FUNC_IS_METHOD
    }

    fn arity(&self) -> haxby_vm::arity::Arity {
        haxby_vm::arity::Arity::required(3)
    }

    fn name(&self) -> &str {
        "_post"
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
            let request = match module.load_named_value("Request") {
                Some(request) => request,
                None => {
                    return LoadResult::error("cannot find Request");
                }
            };

            let request = match request.as_struct() {
                Some(request) => request,
                None => {
                    return LoadResult::error("Request is not a struct");
                }
            };

            request.insert_builtin::<RequestGet>();
            request.insert_builtin::<RequestPost>();

            LoadResult::success()
        }
        None => LoadResult::error("invalid network module"),
    }
}
