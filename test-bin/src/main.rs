// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashSet,
    fmt::Display,
    process::{ExitCode, Termination, exit},
    time::{Duration, Instant},
};

use aria_compiler::compile_from_source;
use aria_parser::ast::SourceBuffer;
use clap::Parser;
use enum_as_inner::EnumAsInner;
use glob::Paths;
use haxby_vm::vm::VirtualMachine;
use rayon::prelude::*;
use regex::Regex;

#[derive(clap::ValueEnum, Clone, Debug, Default)]
enum SortBy {
    #[default]
    Name,
    Duration,
}

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// A glob expression resulting in which test files to run
    #[arg(long)]
    path: String,
    #[arg(long)]
    /// Print additional output information
    verbose: bool,
    #[arg(long)]
    /// Run tests sequentially instead of in parallel
    sequential: bool,
    #[arg(long = "fail-fast")]
    /// Exit when any test fails, instead of running the entire suite
    fail_fast: bool,
    #[arg(long, value_enum, default_value_t)]
    /// Sort test results by name or duration
    sort_by: SortBy,
    /// Skip tests whose file name matches any of these regexes. May repeat.
    #[arg(long = "skip-pattern")]
    skip_pattern: Vec<String>,
}

#[derive(Clone, EnumAsInner)]
enum TestCaseOutcome {
    Pass,
    Fail(String),
    #[allow(dead_code)]
    XFail(String),
}

impl TestCaseOutcome {
    fn result_emoji(&self) -> &'static str {
        match self {
            TestCaseOutcome::Pass => "✅",
            TestCaseOutcome::Fail(_) => "❌",
            TestCaseOutcome::XFail(_) => "⚠️ ",
        }
    }

    fn display_error_reason(&self) -> String {
        if let Some(reason) = self.as_fail() {
            format!("[{}]", reason)
        } else {
            String::new()
        }
    }
}

#[derive(Clone)]
struct TestCaseResult {
    test: String,
    duration: Duration,
    result: TestCaseOutcome,
}

impl TestCaseResult {
    fn pass(test: &str, duration: Duration) -> Self {
        Self {
            test: test.to_owned(),
            duration,
            result: TestCaseOutcome::Pass,
        }
    }

    fn fail(test: &str, duration: Duration, reason: String) -> Self {
        Self {
            test: test.to_owned(),
            duration,
            result: TestCaseOutcome::Fail(reason),
        }
    }

    fn xfail(test: &str, duration: Duration, reason: String) -> Self {
        Self {
            test: test.to_owned(),
            duration,
            result: TestCaseOutcome::XFail(reason),
        }
    }
}

impl Display for TestCaseResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} [in {}.{:03} seconds]",
            self.result.result_emoji(),
            self.test,
            self.result.display_error_reason(),
            self.duration.as_secs(),
            self.duration.subsec_millis()
        )
    }
}

fn should_skip_file_name(path: &std::path::Path, skip_regex: &[Regex]) -> bool {
    let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
        return false;
    };
    skip_regex.iter().any(|re| re.is_match(fname))
}

fn parse_tags_from_file(path: &str) -> HashSet<String> {
    let mut tags = HashSet::new();
    let Ok(text) = std::fs::read_to_string(path) else {
        return tags;
    };

    static TAGS_RE: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"(?i)^\s*###\s*TAGS:\s*(.+)\s*$").unwrap());

    for line in text.lines() {
        if let Some(cap) = TAGS_RE.captures(line)
            && let Some(list) = cap.get(1)
        {
            for t in list.as_str().split(',') {
                let t = t.trim();
                if !t.is_empty() {
                    tags.insert(t.to_ascii_uppercase());
                }
            }
        }
    }
    tags
}

fn run_test_from_pattern(path: &str) -> TestCaseResult {
    let tags = parse_tags_from_file(path);
    let start_wall = Instant::now();

    let run_once = || -> TestCaseResult {
        let start = Instant::now();

        let buffer = match SourceBuffer::file(path) {
            Ok(buffer) => buffer,
            Err(err) => {
                return TestCaseResult::fail(path, start.elapsed(), format!("I/O error: {err}"));
            }
        };

        let entry_cm = match compile_from_source(&buffer, &Default::default()) {
            Ok(m) => m,
            Err(e) => {
                let err_msg = e
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                return TestCaseResult::fail(
                    path,
                    start.elapsed(),
                    format!("compilation error: {err_msg}"),
                );
            }
        };

        let mut vm = VirtualMachine::default();

        let entry_rm = match vm.load_module("", entry_cm) {
            Ok(rle) => match rle {
                haxby_vm::vm::RunloopExit::Ok(m) => m.module,
                haxby_vm::vm::RunloopExit::Exception(e) => {
                    let mut frame = Default::default();
                    let epp = e.value.prettyprint(&mut frame, &mut vm);
                    return TestCaseResult::fail(path, start.elapsed(), epp);
                }
            },
            Err(err) => return TestCaseResult::fail(path, start.elapsed(), err.prettyprint(None)),
        };

        match vm.execute_module(&entry_rm) {
            Ok(rle) => match rle {
                haxby_vm::vm::RunloopExit::Ok(_) => TestCaseResult::pass(path, start.elapsed()),
                haxby_vm::vm::RunloopExit::Exception(e) => {
                    let mut frame = Default::default();
                    let epp = e.value.prettyprint(&mut frame, &mut vm);
                    TestCaseResult::fail(path, start.elapsed(), epp)
                }
            },
            Err(err) => {
                TestCaseResult::fail(path, start.elapsed(), err.prettyprint(Some(entry_rm)))
            }
        }
    };

    let mut outcome = run_once();

    let is_flaky = tags.contains("FLAKEY") || tags.contains("FLAKY");
    if is_flaky && outcome.result.is_fail() {
        outcome = run_once();
    }

    let is_xfail = tags.contains("XFAIL");
    if is_xfail {
        match &outcome.result {
            TestCaseOutcome::Pass => {
                return TestCaseResult::fail(
                    path,
                    start_wall.elapsed(),
                    "unexpected pass (XFAIL)".into(),
                );
            }
            TestCaseOutcome::Fail(reason) => {
                return TestCaseResult::xfail(path, start_wall.elapsed(), reason.clone());
            }
            _ => {
                panic!("test runner should only produce pass/fail")
            }
        }
    }

    outcome
}

#[derive(Default)]
struct SuiteReport {
    passes: Vec<TestCaseResult>,
    fails: Vec<TestCaseResult>,
    xfails: Vec<TestCaseResult>,
    duration: Duration,
}

impl SuiteReport {
    fn num_fails(&self) -> usize {
        self.fails.len()
    }

    fn num_passes(&self) -> usize {
        self.passes.len()
    }

    fn num_xfails(&self) -> usize {
        self.xfails.len()
    }

    fn len(&self) -> usize {
        self.num_fails() + self.num_passes() + self.num_xfails()
    }

    fn pass(&mut self, result: TestCaseResult) {
        self.passes.push(result);
    }

    fn fail(&mut self, result: TestCaseResult) {
        self.fails.push(result);
    }

    fn xfail(&mut self, result: TestCaseResult) {
        self.xfails.push(result);
    }

    fn sort(&mut self, by: &SortBy) -> &mut Self {
        match by {
            SortBy::Name => {
                self.passes.sort_by(|a, b| a.test.cmp(&b.test));
                self.fails.sort_by(|a, b| a.test.cmp(&b.test));
                self.xfails.sort_by(|a, b| a.test.cmp(&b.test));
            }
            SortBy::Duration => {
                self.passes.sort_by(|a, b| a.duration.cmp(&b.duration));
                self.fails.sort_by(|a, b| a.duration.cmp(&b.duration));
                self.xfails.sort_by(|a, b| a.duration.cmp(&b.duration));
            }
        }

        self
    }
}

impl Display for SuiteReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for pass in &self.passes {
            writeln!(f, "{}", pass)?;
        }
        for xfail in &self.xfails {
            writeln!(f, "{}", xfail)?;
        }
        for fail in &self.fails {
            writeln!(f, "{}", fail)?;
        }

        write!(
            f,
            "{} tests total - {} passed, {} failed, {} xfailed - in {}.{:03} seconds",
            self.len(),
            self.num_passes(),
            self.num_fails(),
            self.num_xfails(),
            self.duration.as_secs(),
            self.duration.subsec_millis(),
        )
    }
}

impl Termination for SuiteReport {
    fn report(self) -> ExitCode {
        if self.num_fails() > 0 {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

fn run_tests_from_pattern(patterns: Paths, args: &Args, skip_regex: &[Regex]) -> SuiteReport {
    let mut results = SuiteReport::default();

    let start = Instant::now();

    let outcomes = if args.sequential {
        let mut ret = vec![];
        for pattern in patterns.flatten() {
            if should_skip_file_name(&pattern, skip_regex) {
                continue;
            }

            let test_name = pattern.file_stem().unwrap().to_str().unwrap();
            let test_path = pattern.as_os_str().to_str().unwrap();
            if args.verbose {
                println!("Running {test_name} (at {test_path})");
            }
            let result = run_test_from_pattern(test_path);
            if args.fail_fast && result.result.is_fail() {
                ret.push(result);
                break;
            } else {
                ret.push(result);
            }
        }
        ret
    } else {
        patterns
            .flatten()
            .filter(|p| !should_skip_file_name(p, skip_regex))
            .par_bridge()
            .map(|path| {
                let test_path = path.as_os_str().to_str().unwrap();
                run_test_from_pattern(test_path)
            })
            .collect::<_>()
    };

    results.duration = start.elapsed();

    for result in outcomes {
        match &result.result {
            TestCaseOutcome::Pass => results.pass(result),
            TestCaseOutcome::Fail(_) => {
                results.fail(result);
            }
            TestCaseOutcome::XFail(_) => {
                results.xfail(result);
            }
        }
    }

    results
}

fn main() -> SuiteReport {
    let args = Args::parse();
    if args.fail_fast && !args.sequential {
        println!("--fail-fast is only supported in sequential mode; ignoring");
    }

    let mut skip_regex = Vec::new();
    for pattern in &args.skip_pattern {
        match Regex::new(pattern) {
            Ok(re) => skip_regex.push(re),
            Err(e) => {
                eprintln!("invalid --skip-pattern `{pattern}`: {e}");
                exit(2);
            }
        }
    }

    let mut results = match glob::glob(&args.path) {
        Ok(pattern) => run_tests_from_pattern(pattern, &args, &skip_regex),
        Err(err) => {
            eprintln!("invalid pattern: {err}");
            exit(1);
        }
    };
    if results.num_fails() == 0 && !args.verbose {
        println!("All tests passed; --verbose to print full report");
        exit(0);
    }

    results.sort(&args.sort_by);

    println!("{}", results);

    results
}
