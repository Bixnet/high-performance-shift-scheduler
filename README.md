# High-Performance Shift Scheduler in Rust

A performance-oriented command-line shift scheduler built around tabu search.

## Highlights

- Solves synthetic instances from 50 to 5,000 staff.
- Incremental scoring: only the staff touched by a move are rescored.
- Ring-buffer tabu memory using `VecDeque`.
- Flat assignment storage using contiguous `Vec<usize>` data.
- Parallel neighborhood evaluation with Rayon.
- Criterion benchmarks for 50 / 500 / 5,000 staff.
- GitHub Actions CI for formatting, Clippy, and tests.

## Run

```bash
cargo run --release -- --staff 5000 --days 28 --iterations 250
```

## Benchmarks

```bash
cargo bench
cargo install flamegraph
cargo flamegraph --release -- --staff 5000 --days 28 --iterations 250
```

The intended performance claim is measured only after an apples-to-apples comparison on the same machine:

| Instance | Baseline | Rust | Speedup |
| --- | ---: | ---: | ---: |
| 50 staff | TBD | TBD | TBD |
| 500 staff | TBD | TBD | TBD |
| 5,000 staff | TBD | TBD | TBD |

## Testing

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

## Tech stack

Rust · Rayon · Criterion · cargo-flamegraph · GitHub Actions
