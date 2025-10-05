# Benchmark Suite

This directory contains criterion-based benchmarks for winutils performance testing.

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific benchmark
cargo bench --bench path_normalization

# Save baseline
cargo bench --workspace -- --save-baseline main

# Compare with baseline
cargo bench --workspace -- --baseline main

# Generate detailed reports
cargo bench --workspace -- --output-format bencher
```

## Benchmark Structure

- `path_normalization.rs` - Path handling performance
- `utility_performance.rs` - Individual utility benchmarks
- `integration_benchmarks.rs` - End-to-end workflow benchmarks

## Performance Targets

Maintain 4.68x average improvement over GNU coreutils:

- hashsum: 15.6x
- wc: 12.3x
- sort: 8.7x
- ls: 5.2x
- cat: 3.8x

## CI Integration

Benchmarks run automatically on:

- Pull requests to main (regression detection)
- Pushes to main (baseline updates)
- Manual workflow dispatch

Regression limit: 10% maximum slowdown allowed.
