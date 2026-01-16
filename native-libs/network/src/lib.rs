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
        let this = haxby_vm::builtins::VmGlobals::extract_arg(frame, |x| x.as_object().cloned())?;
        let headers = haxby_vm::builtins::VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        let url_sym = vm
            .globals
            .intern_symbol("url")
            .expect("too many symbols interned");
        let timeout_sym = vm
            .globals
            .intern_symbol("timeout")
            .expect("too many symbols interned");
        let response_sym = vm
            .globals
            .intern_symbol("Response")
            .expect("too many symbols interned");
        let error_sym = vm
            .globals
            .intern_symbol("Error")
            .expect("too many symbols interned");
        let status_code_sym = vm
            .globals
            .intern_symbol("status_code")
            .expect("too many symbols interned");
        let headers_sym = vm
            .globals
            .intern_symbol("headers")
            .expect("too many symbols interned");
        let content_sym = vm
            .globals
            .intern_symbol("content")
            .expect("too many symbols interned");
        let msg_sym = vm
            .globals
            .intern_symbol("msg")
            .expect("too many symbols interned");

        let this_url = this.extract_field(&vm.globals, url_sym, |field: RuntimeValue| {
            field.as_string().cloned()
        })?;
        let this_timeout =
            this.extract_field(&vm.globals, timeout_sym, |field: RuntimeValue| {
                field.as_float().cloned()
            })?;
        let as_struct = this.get_struct();
        let this_response =
            as_struct.extract_field(&vm.globals, response_sym, |field: RuntimeValue| {
                field.as_struct().cloned()
            })?;
        let this_error =
            as_struct.extract_field(&vm.globals, error_sym, |field: RuntimeValue| {
                field.as_struct().cloned()
            })?;

        let mut client = reqwest::blocking::Client::new()
            .get(this_url.raw_value())
            .timeout(std::time::Duration::from_secs_f64(
                *this_timeout.raw_value(),
            ));
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
                let response_obj = RuntimeValue::Object(Object::new(&this_response));
                let _ = response_obj.write_attribute(
                    status_code_sym,
                    haxby_vm::runtime_value::RuntimeValue::Integer(
                        (r.status().as_u16() as i64).into(),
                    ),
                    &mut vm.globals,
                );
                let header_list = List::from(&[]);
                for header in r.headers() {
                    let header_kvp = List::from(&[
                        RuntimeValue::String(header.0.as_str().into()),
                        RuntimeValue::String(header.1.to_str().unwrap_or("<err>").into()),
                    ]);
                    header_list.append(RuntimeValue::List(header_kvp));
                }
                let _ = response_obj.write_attribute(
                    headers_sym,
                    RuntimeValue::List(header_list),
                    &mut vm.globals,
                );
                match r.text() {
                    Ok(content) => {
                        let _ = response_obj.write_attribute(
                            content_sym,
                            RuntimeValue::String(content.into()),
                            &mut vm.globals,
                        );
                    }
                    _ => {
                        let error_obj = RuntimeValue::Object(Object::new(&this_error));
                        let _ = error_obj.write_attribute(
                            msg_sym,
                            RuntimeValue::String("content is not a valid String".into()),
                            &mut vm.globals,
                        );
                        let result_err = vm.globals.create_result_err(error_obj)?;
                        frame.stack.push(result_err);
                        return ExecutionResult::Ok(haxby_vm::vm::RunloopExit::Ok(()));
                    }
                }

                let result_ok = vm.globals.create_result_ok(response_obj.clone())?;

                frame.stack.push(result_ok);
                Ok(haxby_vm::vm::RunloopExit::Ok(()))
            }
            Err(e) => {
                let error_obj = RuntimeValue::Object(Object::new(&this_error));
                let _ = error_obj.write_attribute(
                    msg_sym,
                    RuntimeValue::String(e.to_string().into()),
                    &mut vm.globals,
                );
                let result_err = vm.globals.create_result_err(error_obj)?;

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
        let this = haxby_vm::builtins::VmGlobals::extract_arg(frame, |x| x.as_object().cloned())?;
        let headers = haxby_vm::builtins::VmGlobals::extract_arg(frame, |x| x.as_list().cloned())?;
        let payload =
            haxby_vm::builtins::VmGlobals::extract_arg(frame, |x| x.as_string().cloned())?;

        let url_sym = vm
            .globals
            .intern_symbol("url")
            .expect("too many symbols interned");
        let timeout_sym = vm
            .globals
            .intern_symbol("timeout")
            .expect("too many symbols interned");
        let response_sym = vm
            .globals
            .intern_symbol("Response")
            .expect("too many symbols interned");
        let error_sym = vm
            .globals
            .intern_symbol("Error")
            .expect("too many symbols interned");
        let status_code_sym = vm
            .globals
            .intern_symbol("status_code")
            .expect("too many symbols interned");
        let headers_sym = vm
            .globals
            .intern_symbol("headers")
            .expect("too many symbols interned");
        let content_sym = vm
            .globals
            .intern_symbol("content")
            .expect("too many symbols interned");
        let msg_sym = vm
            .globals
            .intern_symbol("msg")
            .expect("too many symbols interned");

        let this_url = this.extract_field(&vm.globals, url_sym, |field: RuntimeValue| {
            field.as_string().cloned()
        })?;
        let this_timeout =
            this.extract_field(&vm.globals, timeout_sym, |field: RuntimeValue| {
                field.as_float().cloned()
            })?;
        let as_struct = this.get_struct();
        let this_response =
            as_struct.extract_field(&vm.globals, response_sym, |field: RuntimeValue| {
                field.as_struct().cloned()
            })?;
        let this_error =
            as_struct.extract_field(&vm.globals, error_sym, |field: RuntimeValue| {
                field.as_struct().cloned()
            })?;

        let mut client = reqwest::blocking::Client::new()
            .post(this_url.raw_value())
            .body(payload.raw_value().to_owned())
            .timeout(std::time::Duration::from_secs_f64(
                *this_timeout.raw_value(),
            ));
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
                let response_obj = RuntimeValue::Object(Object::new(&this_response));
                let _ = response_obj.write_attribute(
                    status_code_sym,
                    haxby_vm::runtime_value::RuntimeValue::Integer(
                        (r.status().as_u16() as i64).into(),
                    ),
                    &mut vm.globals,
                );
                let header_list = List::from(&[]);
                for header in r.headers() {
                    let header_kvp = List::from(&[
                        RuntimeValue::String(header.0.as_str().into()),
                        RuntimeValue::String(header.1.to_str().unwrap_or("<err>").into()),
                    ]);
                    header_list.append(RuntimeValue::List(header_kvp));
                }
                let _ = response_obj.write_attribute(
                    headers_sym,
                    RuntimeValue::List(header_list),
                    &mut vm.globals,
                );
                match r.text() {
                    Ok(content) => {
                        let _ = response_obj.write_attribute(
                            content_sym,
                            RuntimeValue::String(content.into()),
                            &mut vm.globals,
                        );
                    }
                    _ => {
                        let error_obj = RuntimeValue::Object(Object::new(&this_error));
                        let _ = error_obj.write_attribute(
                            msg_sym,
                            RuntimeValue::String("content is not a valid String".into()),
                            &mut vm.globals,
                        );
                        let result_err = vm.globals.create_result_err(error_obj)?;

                        frame.stack.push(result_err);
                        return ExecutionResult::Ok(haxby_vm::vm::RunloopExit::Ok(()));
                    }
                }

                let result_ok = vm.globals.create_result_ok(response_obj.clone())?;

                frame.stack.push(result_ok);
                Ok(haxby_vm::vm::RunloopExit::Ok(()))
            }
            Err(e) => {
                let error_obj = RuntimeValue::Object(Object::new(&this_error));
                let _ = error_obj.write_attribute(
                    msg_sym,
                    RuntimeValue::String(e.to_string().into()),
                    &mut vm.globals,
                );
                let result_err = vm.globals.create_result_err(error_obj)?;

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
    vm: *mut haxby_vm::vm::VirtualMachine,
    module: *const RuntimeModule,
) -> LoadResult {
    match unsafe { (vm.as_mut(), module.as_ref()) } {
        (Some(vm), Some(module)) => {
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

            request.insert_builtin::<RequestGet>(&mut vm.globals);
            request.insert_builtin::<RequestPost>(&mut vm.globals);

            LoadResult::success()
        }
        _ => LoadResult::error("invalid network module"),
    }
}
