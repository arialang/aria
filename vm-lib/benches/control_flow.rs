// SPDX-License-Identifier: Apache-2.0
use std::hint::black_box;

use aria_compiler::compile_from_source;
use aria_parser::ast::SourceBuffer;
use criterion::{Criterion, criterion_group, criterion_main};
use haxby_vm::haxby_eval;

fn bench_aria_code_aux(bench_name: &str, src: &str, c: &mut Criterion) {
    c.bench_function(&format!("{}/compile", bench_name), |b| {
        b.iter(|| {
            let sb = SourceBuffer::stdin(src);
            black_box(
                compile_from_source(&sb, &Default::default()).expect("module did not compile"),
            );
        })
    });

    let sb = SourceBuffer::stdin(src);

    c.bench_function(&format!("{}/eval", bench_name), |b| {
        b.iter_batched(
            || compile_from_source(&sb, &Default::default()).expect("module did not compile"),
            |module| black_box(haxby_eval(module, Default::default()).unwrap()),
            criterion::BatchSize::SmallInput,
        )
    });
}

fn bench_if(c: &mut Criterion) {
    const INPUT: &str = r#"
    func main() {
        if true {

        } else {

        }
        if false {

        } else {

        }
    }
    "#;

    bench_aria_code_aux("control_flow/if", INPUT, c);
}

fn bench_for(c: &mut Criterion) {
    const INPUT: &str = r#"
    func main() {
        for i in Range.from(0).to(10) {
            println(i);
        }
    }
    "#;

    bench_aria_code_aux("control_flow/for", INPUT, c);
}

fn bench_while(c: &mut Criterion) {
    const INPUT: &str = r#"
    func main() {
        val i = 0;
        while i < 10 {
            i = i + 1;
        }
    }
    "#;

    bench_aria_code_aux("control_flow/while", INPUT, c);
}

fn bench_empty_function_call(c: &mut Criterion) {
    const INPUT: &str = r#"
    func foo() {}
    func main() {
        val i = 0;
        while i < 10 {
            foo();
            i += 1;
        }
    }
    "#;

    bench_aria_code_aux("control_flow/empty_function_call", INPUT, c);
}

criterion_group!(
    control_flow,
    bench_if,
    bench_for,
    bench_while,
    bench_empty_function_call
);
criterion_main!(control_flow);
