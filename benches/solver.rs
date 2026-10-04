use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use shift_scheduler::{Problem, Solver, SolverConfig};

fn bench_solver(c: &mut Criterion) {
    let mut group = c.benchmark_group("tabu-scheduler");
    group.sample_size(10);

    for &staff in &[50usize, 500, 5_000] {
        let problem = Problem::synthetic(staff, 28, 42);
        let solver = Solver::new(SolverConfig {
            iterations: 50,
            candidates_per_slot: 4,
            tabu_capacity: 256,
            seed: 42,
        });

        group.bench_with_input(BenchmarkId::from_parameter(staff), &staff, |b, _| {
            b.iter(|| {
                let result = solver.solve(black_box(&problem));
                black_box(result.score);
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_solver);
criterion_main!(benches);
