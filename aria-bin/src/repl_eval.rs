// SPDX-License-Identifier: Apache-2.0
use std::ops::DerefMut;

use aria_compiler::{CompilationOptions, compile_from_ast};
use aria_parser::ast::{
    ExpressionStatement, SourceBuffer, TopLevelEntry,
    prettyprint::{PrettyPrintable, printout_accumulator::PrintoutAccumulator},
    source_to_ast,
};
use haxby_vm::{
    runtime_module::RuntimeModule,
    vm::{VirtualMachine, VmOptions},
};
use reedline::{DefaultPrompt, Reedline, Validator};

use crate::{
    Args,
    error_reporting::{
        build_report_from_compiler_error, build_report_from_parser_error,
        build_report_from_vm_error, build_report_from_vm_exception,
        print_report_from_compiler_error, print_report_from_parser_error,
        print_report_from_vm_error, print_report_from_vm_exception,
    },
};

struct ReplValidator;
impl Validator for ReplValidator {
    fn validate(&self, line: &str) -> reedline::ValidationResult {
        use reedline::ValidationResult::{Complete, Incomplete};

        let mut quote: Option<char> = None; // '" or '\''
        let mut escaped = false;
        let mut balance: Vec<char> = Vec::new();
        let mut comment = false;

        for c in line.chars() {
            // inside string
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                    continue;
                }
                if c == '\\' {
                    escaped = true;
                    continue;
                }
                if c == q {
                    quote = None;
                }
                continue;
            } else if c == '#' || comment {
                comment = c != '\n';
                continue;
            } else {
                match c {
                    '"' | '\'' => quote = Some(c),
                    '(' => balance.push(')'),
                    '[' => balance.push(']'),
                    '{' => balance.push('}'),
                    ')' | ']' | '}' => {
                        if !matches!(balance.last(), Some(&need) if need == c) {
                            return Complete;
                        } else {
                            balance.pop();
                        }
                    }
                    _ => {}
                }
            }
        }

        if quote.is_none() && balance.is_empty() {
            Complete
        } else {
            Incomplete
        }
    }
}

struct LineEditor {
    line_editor: Reedline,
    prompt: DefaultPrompt,
}

impl LineEditor {
    pub fn new() -> Self {
        let validator = Box::new(ReplValidator);
        let prompt = DefaultPrompt {
            left_prompt: reedline::DefaultPromptSegment::Empty,
            right_prompt: reedline::DefaultPromptSegment::Empty,
        };
        #[cfg(test)]
        let line_editor = Reedline::create().with_validator(validator);
        #[cfg(not(test))]
        let line_editor = Reedline::create()
            .with_validator(validator)
            .with_history(Box::new(
                reedline::FileBackedHistory::with_file(1024, "history.aria".into())
                    .expect("history"),
            ));
        Self {
            line_editor,
            prompt,
        }
    }
}

impl LineEditor {
    fn read_input(&mut self) -> (String, bool) {
        let sig = self.line_editor.read_line(&self.prompt);
        match sig {
            Ok(reedline::Signal::Success(buffer)) => (buffer, false),
            Ok(reedline::Signal::CtrlC) | Ok(reedline::Signal::CtrlD) | Err(_) => {
                (String::new(), true)
            }
        }
    }
}

fn is_call_to_print_or_println(expr: &ExpressionStatement) -> bool {
    if let Some(v) = &expr.val {
        let (is_call, name) = v.is_function_call();
        return is_call && matches!(name, Some("print") | Some("println"));
    }
    false
}

fn massage_ast_for_repl(ast: &mut aria_parser::ast::ParsedModule) -> bool {
    if ast.entries.is_empty() {
        return false;
    }
    let idx = ast.entries.len() - 1;

    let TopLevelEntry::ExpressionStatement(expr) = &ast.entries[idx] else {
        return false;
    };
    if is_call_to_print_or_println(expr) {
        return false;
    }
    let Some(val) = &expr.val else { return false };

    let new_node = val.call_function_passing_me("println");
    ast.entries[idx] = TopLevelEntry::ExpressionStatement(ExpressionStatement {
        loc: val.loc().clone(),
        val: Some(new_node),
    });
    true
}

#[cfg(test)]
pub struct ReplStepResult {
    pub stdout: String,
    #[allow(dead_code)]
    pub stderr: String,
    pub ok: bool,
}

pub struct Repl<'a> {
    vm: VirtualMachine,
    module: RuntimeModule,
    args: &'a Args,
    counter: u64,
}

impl<'a> Repl<'a> {
    #[allow(clippy::unit_arg)]
    pub fn new(vm_options: VmOptions, args: &'a Args) -> Result<Self, ()> {
        let mut vm = VirtualMachine::with_options(vm_options);
        let repl_module_preamble = if args.no_repl_preamble {
            ""
        } else {
            include_str!("repl_preamble.aria")
        };

        let sb = SourceBuffer::stdin_with_name(repl_module_preamble, "repl");
        let ast = match source_to_ast(&sb) {
            Ok(ast) => ast,
            Err(err) => {
                return Err(print_report_from_parser_error(&err));
            }
        };

        let comp_opts = CompilationOptions::default();

        let c_module = match compile_from_ast(&ast, &comp_opts) {
            Ok(module) => module,
            Err(err) => {
                err.iter().for_each(print_report_from_compiler_error);
                return Err(());
            }
        };

        let r_module = RuntimeModule::new(c_module);

        let r_module = match vm.load_into_module("repl", r_module) {
            Ok(rle) => match rle {
                haxby_vm::vm::RunloopExit::Ok(m) => m.module,
                haxby_vm::vm::RunloopExit::Exception(exc) => {
                    return Err(print_report_from_vm_exception(&mut vm, &exc));
                }
            },
            Err(err) => {
                return Err(print_report_from_vm_error(&err));
            }
        };

        vm.inject_imported_module("repl", r_module.clone());
        Ok(Repl {
            vm,
            module: r_module,
            args,
            counter: 0,
        })
    }

    fn try_parse_source(
        &mut self,
        buffer: &SourceBuffer,
    ) -> Result<aria_parser::ast::ParsedModule, aria_parser::ast::ParserError> {
        source_to_ast(buffer)
    }

    #[allow(clippy::unit_arg)]
    pub fn process_buffer(&mut self, buffer: &str) -> Result<RuntimeModule, ()> {
        let module_name: String = format!("__repl_chunk_{}", self.counter);
        self.counter += 1;

        let module_source_code = format!("import * from repl;\n{}\n", buffer);
        let sb = SourceBuffer::stdin_with_name(&module_source_code, &module_name);

        let mut parsed_source = self.try_parse_source(&sb);
        if parsed_source.is_err() && !(buffer.ends_with('}') || buffer.ends_with(';')) {
            let buffer_with_semicolon = format!("{};", module_source_code);
            let sb2 = SourceBuffer::stdin_with_name(&buffer_with_semicolon, &module_name);
            let parsed_source2 = self.try_parse_source(&sb2);
            if parsed_source2.is_ok() {
                parsed_source = parsed_source2;
            }
        }

        let mut ast = match parsed_source {
            Ok(ast) => ast,
            Err(err) => {
                return Err(self.print_error_report(build_report_from_parser_error(&err)));
            }
        };

        let mutated = massage_ast_for_repl(&mut ast);

        if self.args.dump_ast {
            let ast_buffer = PrintoutAccumulator::default();
            let output = ast.prettyprint(ast_buffer).value();
            println!("AST dump:\n{output}\n");
            if mutated {
                println!("note: AST mutated for REPL purposes");
            }
        }

        let comp_opts = CompilationOptions::default();

        let c_module = match compile_from_ast(&ast, &comp_opts) {
            Ok(module) => module,
            Err(err) => {
                err.iter()
                    .for_each(|e| self.print_error_report(build_report_from_compiler_error(e)));
                return Err(());
            }
        };

        if self.args.dump_mod {
            let mod_buffer = PrintoutAccumulator::default();
            let output = c_module.prettyprint(mod_buffer).value();
            println!("Module dump:\n{output}\n");
        }

        let r_module = RuntimeModule::new(c_module);
        if r_module
            .lift_all_symbols_from_other(&self.module, &self.vm)
            .is_err()
        {
            return Err(());
        }
        let load_result = self.vm.load_into_module("repl", r_module);
        match load_result {
            Ok(rle) => match rle {
                haxby_vm::vm::RunloopExit::Ok(m) => {
                    let new_module = m.module;
                    let _ = self
                        .module
                        .lift_all_symbols_from_other(&new_module, &self.vm);
                    Ok(new_module)
                }
                haxby_vm::vm::RunloopExit::Exception(exc) => {
                    let report = build_report_from_vm_exception(&mut self.vm, &exc);
                    Err(self.print_error_report(report))
                }
            },
            Err(err) => Err(self.print_error_report(build_report_from_vm_error(&err))),
        }
    }

    fn print_error_report(&mut self, report: crate::error_reporting::PrintableReport<'_>) {
        let console_rc = self.vm.console();
        let mut console_borrow = console_rc.borrow_mut();
        let console = console_borrow.deref_mut();
        let _ = report.0.write(report.1, console);
    }

    #[cfg(test)]
    pub fn eval_line(&mut self, src: &str) -> ReplStepResult {
        fn diff(before: &str, after: &str) -> String {
            if after.len() >= before.len() {
                after[before.len()..].to_string()
            } else {
                String::new()
            }
        }

        let before_console = self
            .vm
            .console()
            .borrow()
            .as_any()
            .downcast_ref::<haxby_vm::console::TestConsole>()
            .unwrap()
            .clone();
        let before_stdout = before_console.stdout.clone();
        let before_stderr = before_console.stderr.clone();

        let ok = self.process_buffer(src).is_ok();

        let after_console = self
            .vm
            .console()
            .borrow()
            .as_any()
            .downcast_ref::<haxby_vm::console::TestConsole>()
            .unwrap()
            .clone();
        let after_stdout = after_console.stdout.clone();
        let after_stderr = after_console.stderr.clone();

        ReplStepResult {
            stdout: diff(&before_stdout, &after_stdout),
            stderr: diff(&before_stderr, &after_stderr),
            ok,
        }
    }
}

pub(crate) fn repl_eval(args: &Args) -> i32 {
    if args.perf_trace_dest.is_some() {
        eprintln!("Warning: --perf-trace-dest has no effect in REPL mode");
    }

    let vm_opts = VmOptions::from(args);
    let mut repl = Repl::new(vm_opts, args).unwrap();

    let mut ed = LineEditor::new();

    loop {
        let (input, eof) = ed.read_input();

        if eof {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        } else if input == ":quit" {
            break;
        } else {
            let _ = repl.process_buffer(input);
        }
    }

    0 // REPL will always exit 0
}
