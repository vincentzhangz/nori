use criterion::{Criterion, black_box, criterion_group, criterion_main};
use nori::{CompileOptions, compile_source};

fn compile_todo(c: &mut Criterion) {
    let source = include_str!("../../../examples/Todo.nori");
    c.bench_function("compile Todo.nori x1", |b| {
        b.iter(|| {
            compile_source(
                black_box(source),
                CompileOptions {
                    filename: "Todo.nori".to_string(),
                    ..CompileOptions::default()
                },
            )
            .unwrap()
        })
    });

    c.bench_function("compile Todo.nori x1000 (batch)", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                let _ = compile_source(
                    black_box(source),
                    CompileOptions {
                        filename: "Todo.nori".to_string(),
                        ..CompileOptions::default()
                    },
                )
                .unwrap();
            }
        })
    });
}

criterion_group!(benches, compile_todo);
criterion_main!(benches);
