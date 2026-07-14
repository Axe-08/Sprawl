use std::hint::black_box;
use criterion::{criterion_group, criterion_main, Criterion};
use sprawl_sweeper::safety_gate::{SafetyGate, Ecosystem};
use std::path::PathBuf;

fn bench_safety_gate(c: &mut Criterion) {
    let gate = SafetyGate::new();
    
    // We mock a PathBuf to pass into nuke_eligible. In reality this touches the filesystem,
    // so this benchmark will measure both the overhead of SafetyGate and some minimal I/O.
    let path = PathBuf::from(".");
    
    c.bench_function("safety_gate_verify", |b| {
        b.iter(|| {
            // black_box prevents the compiler from optimizing away the call
            let _result = gate.verify(black_box(&path), &Ecosystem::Rust);
        })
    });
}

criterion_group!(benches, bench_safety_gate);
criterion_main!(benches);
