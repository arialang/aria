// SPDX-License-Identifier: Apache-2.0
use aria_compiler::{CompilationOptions, compile_from_ast};
use aria_parser::ast::{
    SourceBuffer,
    prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    source_to_ast,
};
use haxby_vm::{
    runtime_module::RuntimeModule,
    vm::{VirtualMachine, VmOptions},
};

use crate::{
    Args,
    error_reporting::{
        print_report_from_compiler_error, print_report_from_parser_error,
        print_report_from_vm_error, print_report_from_vm_exception,
    },
};

impl From<&Args> for CompilationOptions {
    fn from(value: &Args) -> Self {
        CompilationOptions {
            optimize: !value.disable_optimizer,
            dump_builder: value.dump_ir,
        }
    }
}

// To permit return Err(report_blah(x)) where report_blah(x) -> ()
#[allow(clippy::unit_arg)]
fn eval_buffer(
    sb: SourceBuffer,
    vm: &mut VirtualMachine,
    args: &Args,
) -> Result<RuntimeModule, ()> {
    let ast = match source_to_ast(&sb) {
        Ok(ast) => ast,
        Err(err) => {
            return Err(print_report_from_parser_error(&err));
        }
    };

    if args.dump_ast {
        let ast_buffer = PrintoutAccumulator::default();
        let output = ast.prettyprint(ast_buffer).value();
        println!("AST dump:\n{output}\n");
    }

    let comp_opts = CompilationOptions::from(args);

    let c_module = match compile_from_ast(&ast, &comp_opts) {
        Ok(module) => module,
        Err(err) => {
            err.iter().for_each(print_report_from_compiler_error);
            return Err(());
        }
    };

    if args.dump_mod {
        let mod_buffer = PrintoutAccumulator::default();
        let output = c_module.prettyprint(mod_buffer).value();
        println!("Module dump:\n{output}\n");
    }

    let r_module = RuntimeModule::new(c_module);

    let r_module = match vm.load_into_module("", r_module) {
        Ok(rle) => match rle {
            haxby_vm::vm::RunloopExit::Ok(m) => m.module,
            haxby_vm::vm::RunloopExit::Exception(exc) => {
                return Err(print_report_from_vm_exception(vm, &exc));
            }
        },
        Err(err) => {
            return Err(print_report_from_vm_error(&err));
        }
    };

    let exec_result = vm.execute_module(&r_module);

    match exec_result {
        Ok(rle) => match rle {
            haxby_vm::vm::RunloopExit::Ok(_) => Ok(r_module),
            haxby_vm::vm::RunloopExit::Exception(exc) => {
                Err(print_report_from_vm_exception(vm, &exc))
            }
        },
        Err(err) => Err(print_report_from_vm_error(&err)),
    }
}

pub(crate) fn file_eval(path: &str, args: &Args) -> i32 {
    let mut vm = VirtualMachine::with_options(VmOptions::from(args));

    let buffer = SourceBuffer::file(path);
    match buffer {
        Ok(src) => match eval_buffer(src, &mut vm, args) {
            Ok(_) => 0,
            Err(_) => 1,
        },
        Err(err) => {
            println!("error reading source file: {err}");
            1
        }
    }
}
