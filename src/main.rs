use clap::Parser;
use shift_scheduler::{Problem, Solver, SolverConfig};
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(name = "shift-scheduler")]
#[command(about = "Tabu-search shift scheduler for large staffing instances")]
struct Args {
    #[arg(long, default_value_t = 500)]
    staff: usize,
    #[arg(long, default_value_t = 28)]
    days: usize,
    #[arg(long, default_value_t = 250)]
    iterations: usize,
    #[arg(long, default_value_t = 8)]
    candidates_per_slot: usize,
    #[arg(long, default_value_t = 42)]
    seed: u64,
}

fn main() {
    let args = Args::parse();
    let problem = Problem::synthetic(args.staff, args.days, args.seed);
    let solver = Solver::new(SolverConfig {
        iterations: args.iterations,
        candidates_per_slot: args.candidates_per_slot,
        tabu_capacity: 256,
        seed: args.seed,
    });

    let started = Instant::now();
    let result = solver.solve(&problem);
    let elapsed = started.elapsed();

    println!("High-Performance Shift Scheduler");
    println!("staff:       {}", args.staff);
    println!("days:        {}", args.days);
    println!("iterations:  {}", result.iterations);
    println!("best score:  {}", result.score);
    println!("solve time:  {:.3}s", elapsed.as_secs_f64());
}
