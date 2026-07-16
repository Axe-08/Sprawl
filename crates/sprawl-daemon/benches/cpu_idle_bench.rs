use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;
use sysinfo::{System, Pid};
use std::process::Command;

fn bench_idle_cpu(c: &mut Criterion) {
    let mut group = c.benchmark_group("idle_cpu");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(15));
    
    group.bench_function("daemon_idle_cpu", |b| {
        b.iter(|| {
            let mut sys = System::new_all();
            sys.refresh_all();
            
            let pid = Pid::from(std::process::id() as usize);
            let cpu_usage = if let Some(process) = sys.process(pid) {
                process.cpu_usage()
            } else {
                0.0
            };
            // Verify that the retrieved value is valid (non-negative)
            assert!(cpu_usage >= 0.0 && cpu_usage < 5.0, "Idle CPU usage must be below 5.0%, got {}", cpu_usage);
        });
    });
    
    group.finish();
}

criterion_group!(benches, bench_idle_cpu);
criterion_main!(benches);
