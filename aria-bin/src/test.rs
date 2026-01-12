// SPDX-License-Identifier: Apache-2.0

use std::{cell::RefCell, rc::Rc};

use haxby_vm::console::TestConsole;

use crate::{Args, repl_eval::Repl};

fn build_test_repl<'a>(cmdline_options: &'a Args) -> Repl<'a> {
    let console = Rc::new(RefCell::new(TestConsole::default()));
    let vm_options = haxby_vm::vm::VmOptions {
        console: console.clone(),
        ..Default::default()
    };
    Repl::new(vm_options, cmdline_options).unwrap()
}

fn run_check_repl_line(
    repl: &mut Repl,
    line: &str,
    ok: bool,
    must_include_stdout: &[&str],
    must_include_stderr: &[&str],
) {
    let diff = repl.eval_line(line);

    assert!(diff.ok == ok);

    for expected in must_include_stdout {
        assert!(
            diff.stdout.contains(expected),
            "stdout ( {} ) did not contain expected ( {} )",
            diff.stdout,
            expected
        );
    }

    for expected in must_include_stderr {
        assert!(
            diff.stderr.contains(expected),
            "stderr ( {} ) did not contain expected ( {} )",
            diff.stderr,
            expected
        );
    }
}

fn run_passing_repl_line(repl: &mut Repl, line: &str, must_include_stdout: &[&str]) {
    run_check_repl_line(repl, line, true, must_include_stdout, &[])
}

#[test]
fn repl_can_print_integers() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "42;", &["42"]);
    run_passing_repl_line(&mut repl, "3 + 4;", &["7"]);
}

#[test]
fn repl_can_call_functions() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "func foo(x) { return x + 1; }", &[]);
    run_passing_repl_line(&mut repl, "foo(12);", &["13"]);
}

#[test]
fn repl_can_define_structs() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(
        &mut repl,
        r#"
struct Pair {
        type func new(x,y) {
            return alloc(This) {
                .x = x, .y = y,
            };
        }
}
    "#,
        &[],
    );

    run_passing_repl_line(&mut repl, "val p = Pair.new(4,5);", &[]);

    run_passing_repl_line(
        &mut repl,
        r#"
extension Pair {
        func prettyprint() {
            return "Pair({0},{1})".format(this.x,this.y);
        }
}
    "#,
        &[],
    );

    run_passing_repl_line(&mut repl, "p;", &["Pair(4,5)"]);
}

#[test]
fn repl_can_use_if_statement() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "val x = 4;", &[]);
    run_passing_repl_line(&mut repl, "if (x > 2) { println(x + 1); }", &["5"]);
    run_passing_repl_line(&mut repl, "if (x > 4) { println(x + 1); }", &[""]);
}

#[test]
fn repl_can_use_for_statement() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "val l = [1,2,3,4,5,6];", &[]);
    run_passing_repl_line(
        &mut repl,
        "for i in l { println(i); }",
        &["1", "2", "3", "4", "5", "6"],
    );
}

#[test]
fn repl_can_use_while_statement() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "val n = 10;", &[]);
    run_passing_repl_line(
        &mut repl,
        "while n > 0 { n -= 2; println(n); }",
        &["8", "6", "4", "2", "0"],
    );
}

#[test]
fn repl_can_use_match_statement() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "val n = 10;", &[]);

    run_passing_repl_line(
        &mut repl,
        r#"
match n {
        > 10 => {
            println("Greater than 10");
        }
} else {
        println("10 or less");
}
"#,
        &["10 or less"],
    );
}

#[test]
fn repl_printout_maybe() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(
        &mut repl,
        r#"
struct Pair {
    type func new(x,y) {
        return alloc(This) {
            .x = x, .y = y,
        };
    }
    func prettyprint() {
        return "Pair({0},{1})".format(this.x,this.y);
    }
}
"#,
        &[""],
    );

    run_passing_repl_line(
        &mut repl,
        "Maybe::Some(Pair.new(4,5));",
        &["Some(Pair(4,5))"],
    );
    run_passing_repl_line(&mut repl, "Maybe::None;", &["None"]);
}

#[test]
fn repl_adds_semicolon() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "val x = 1", &[]);
    run_passing_repl_line(&mut repl, "val y = 2", &[]);
    run_passing_repl_line(&mut repl, "2 * y + x", &["5"]);
}

#[test]
fn repl_preamble_works() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "3.pow(2)", &["9"]);
    run_passing_repl_line(&mut repl, "val m = Map.new(); m[1] = 'one'; m[1]", &["one"]);
    run_passing_repl_line(&mut repl, "m.get(4)", &["None"]);
}

#[test]
fn repl_skips_preamble() {
    let cmdline_options = Args {
        no_repl_preamble: true,
        ..Default::default()
    };
    let mut repl = build_test_repl(&cmdline_options);

    run_check_repl_line(
        &mut repl,
        "3.pow(2)",
        false,
        &["identifier 'pow' not found"],
        &[],
    );
}

#[test]
fn repl_op_count_error() {
    let cmdline_options = Args {
        no_repl_preamble: true,
        ..Default::default()
    };
    let mut repl = build_test_repl(&cmdline_options);

    run_check_repl_line(
        &mut repl,
        "struct Foo { operator[]=() { return 1; } }",
        false,
        &["operator []= accepts at least 1 arguments, but 0 were declared"],
        &[],
    );

    run_check_repl_line(
        &mut repl,
        "struct Foo { operator + (a,b,c) { return 1; } }",
        false,
        &["operator + accepts exactly 1 arguments, but 3 were declared"],
        &[],
    );
}

#[test]
fn repl_test_printf() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(&mut repl, "'hello = {0}'.printf(42);", &["hello = 42"]);
    run_passing_repl_line(
        &mut repl,
        "'hello = {0} hi = {1}\n'.printf(42, 43);",
        &["hello = 42 hi = 43\n"],
    );
}

#[test]
fn repl_test_invalid_literal() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_check_repl_line(
        &mut repl,
        "0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        false,
        &["0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF is not a valid literal"],
        &[],
    );
}

#[test]
fn repl_allows_comments() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(
        &mut repl,
        r#"
func foo() {
# this is a comment
return # this is another comment
 (
    12345 # comment after expression
 );
}
"#,
        &[],
    );
    run_passing_repl_line(&mut repl, "foo();", &["12345"]);
}

#[test]
fn repl_handles_hashtag_in_string() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(
        &mut repl,
        r#"
func foo() {
# this is a comment
return '#arialang';
}
"#,
        &[],
    );
    run_passing_repl_line(&mut repl, "foo();", &["#arialang"]);
}

#[test]
fn repl_test_oneof_from_empty_list() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_check_repl_line(
        &mut repl,
        r#"
import XorshiftRng from aria.rng.xorshift;
val x = XorshiftRng.new();
x.one_of([])
        "#,
        false,
        &["operation failed: cannot choose from empty list"],
        &[],
    );
}

#[test]
fn repl_includes_ranges() {
    let cmdline_options = Args::default();
    let mut repl = build_test_repl(&cmdline_options);

    run_passing_repl_line(
        &mut repl,
        r#"
val x = 0;
for i in 0.to(5) { x += i; }
"i = {0}\n".printf(x);
"#,
        &["i = 10"],
    );
}
