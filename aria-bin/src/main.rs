// SPDX-License-Identifier: Apache-2.0
mod error_reporting;
mod file_eval;
mod repl_eval;

#[cfg(test)]
mod test;

use clap::Parser;
use haxby_vm::vm::{VirtualMachine, VmOptions};

#[derive(Default, Parser, Debug)]
#[command(author, name = "aria", version = env!("CARGO_PKG_VERSION"), about, trailing_var_arg = true)]
struct Args {
    /// The name of the program file to run
    path: Option<String>,
    /// Should the VM trace instruction execution
    #[arg(long("trace-exec"))]
    trace_exec: bool,
    /// Should the VM dump the stack at each instruction
    #[arg(long("trace-stack"))]
    trace_stack: bool,
    /// Should the AST be dumped after parsing
    #[arg(long("dump-ast"))]
    dump_ast: bool,
    /// Dump the compiler's intermediate representation
    #[arg(long("dump-ir"))]
    dump_ir: bool,
    /// Should the module be dumped after compilation
    #[arg(long("dump-module"))]
    dump_mod: bool,
    /// Turn off compile-time optimizations
    #[arg(long("disable-optimizer"))]
    disable_optimizer: bool,
    #[arg(trailing_var_arg = true)]
    extra_args: Vec<String>,
    #[arg(long("print-lib-path"))]
    print_lib_path: bool,
    /// Turn off REPL preamble
    #[arg(long("no-repl-preamble"))]
    no_repl_preamble: bool,
}

impl From<&Args> for VmOptions {
    fn from(value: &Args) -> Self {
        let mut options = VmOptions::default();

        if value.trace_exec {
            options.tracing = true;
            if value.trace_stack {
                options.dump_stack = true;
            }
        }

        options.vm_args = value.extra_args.clone();

        options
    }
}

fn print_lib_paths() {
    let lib_paths = VirtualMachine::get_aria_library_paths();
    for path in lib_paths {
        println!("{}", path.display());
    }
}

fn main_loop() -> i32 {
    let args = Args::parse();

    if args.print_lib_path {
        print_lib_paths();
        return 0;
    }

    if let Some(path) = &args.path {
        file_eval::file_eval(path, &args)
    } else {
        repl_eval::repl_eval(&args)
    }
}

fn main() {
    let exit_code = main_loop();
    std::process::exit(exit_code);
}
