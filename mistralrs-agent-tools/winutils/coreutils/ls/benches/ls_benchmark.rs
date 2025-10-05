//! Benchmarks for the Windows-optimized ls utility

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Create a temporary directory with a specified number of files
fn create_test_directory(num_files: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create regular files
    for i in 0..num_files {
        let filename = format!("file_{:06}.txt", i);
        let content = format!("Content of file {}", i);
        fs::write(temp_path.join(filename), content).unwrap();
    }

    // Create some directories
    let num_dirs = num_files / 10;
    for i in 0..num_dirs {
        let dirname = format!("dir_{:06}", i);
        fs::create_dir(temp_path.join(dirname)).unwrap();
    }

    temp_dir
}

/// Benchmark basic directory listing performance
fn bench_basic_listing(c: &mut Criterion) {
    let mut group = c.benchmark_group("basic_listing");

    for size in [10, 100, 1000, 5000].iter() {
        let temp_dir = create_test_directory(*size);
        let path = temp_dir.path().to_str().unwrap();

        group.bench_with_input(
            BenchmarkId::new("our_ls", size),
            size,
            |b, _| {
                b.iter(|| {
                    let output = Command::new("cargo")
                        .args(&["run", "--release", "--", path])
                        .current_dir(".")
                        .output()
                        .expect("Failed to run ls");
                    black_box(output);
                });
            },
        );

        // Compare with Windows DIR command
        group.bench_with_input(
            BenchmarkId::new("windows_dir", size),
            size,
            |b, _| {
                b.iter(|| {
                    let output = Command::new("cmd")
                        .args(&["/C", "dir", "/B", path])
                        .output()
                        .expect("Failed to run dir");
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark long format listing performance
fn bench_long_format(c: &mut Criterion) {
    let mut group = c.benchmark_group("long_format");

    for size in [100, 1000].iter() {
        let temp_dir = create_test_directory(*size);
        let path = temp_dir.path().to_str().unwrap();

        group.bench_with_input(
            BenchmarkId::new("our_ls_long", size),
            size,
            |b, _| {
                b.iter(|| {
                    let output = Command::new("cargo")
                        .args(&["run", "--release", "--", "-l", path])
                        .current_dir(".")
                        .output()
                        .expect("Failed to run ls -l");
                    black_box(output);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("windows_dir_detailed", size),
            size,
            |b, _| {
                b.iter(|| {
                    let output = Command::new("cmd")
                        .args(&["/C", "dir", path])
                        .output()
                        .expect("Failed to run dir");
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark recursive listing performance
fn bench_recursive_listing(c: &mut Criterion) {
    let mut group = c.benchmark_group("recursive_listing");

    // Create nested directory structure
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    // Create 3 levels deep with files at each level
    for level in 0..3 {
        let mut current_path = temp_path.to_path_buf();
        for l in 0..=level {
            current_path.push(format!("level_{}", l));
            if l == level {
                fs::create_dir_all(&current_path).unwrap();
                // Add files at this level
                for i in 0..50 {
                    let filename = format!("file_{}_{}.txt", level, i);
                    fs::write(current_path.join(filename), format!("content {}-{}", level, i)).unwrap();
                }
            }
        }
    }

    let path = temp_path.to_str().unwrap();

    group.bench_function("our_ls_recursive", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", "-R", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls -R");
            black_box(output);
        });
    });

    group.bench_function("windows_dir_recursive", |b| {
        b.iter(|| {
            let output = Command::new("cmd")
                .args(&["/C", "dir", "/S", "/B", path])
                .output()
                .expect("Failed to run dir /S");
            black_box(output);
        });
    });

    group.finish();
}

/// Benchmark JSON output performance
fn bench_json_output(c: &mut Criterion) {
    let temp_dir = create_test_directory(1000);
    let path = temp_dir.path().to_str().unwrap();

    c.bench_function("json_output", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", "-j", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls -j");
            black_box(output);
        });
    });
}

/// Benchmark Windows attributes retrieval
fn bench_windows_attributes(c: &mut Criterion) {
    let temp_dir = create_test_directory(500);
    let path = temp_dir.path().to_str().unwrap();

    c.bench_function("windows_attributes", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", "-w", "-l", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls -w -l");
            black_box(output);
        });
    });
}

/// Benchmark parallel processing with different worker counts
fn bench_parallel_workers(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_workers");
    let temp_dir = create_test_directory(2000);
    let path = temp_dir.path().to_str().unwrap();

    for workers in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("workers", workers),
            workers,
            |b, &workers| {
                b.iter(|| {
                    let output = Command::new("cargo")
                        .args(&[
                            "run", "--release", "--",
                            "--workers", &workers.to_string(),
                            path
                        ])
                        .current_dir(".")
                        .output()
                        .expect("Failed to run ls with workers");
                    black_box(output);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark sorting performance
fn bench_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("sorting");
    let temp_dir = create_test_directory(1000);
    let path = temp_dir.path().to_str().unwrap();

    group.bench_function("alphabetical_sort", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls");
            black_box(output);
        });
    });

    group.bench_function("time_sort", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", "-t", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls -t");
            black_box(output);
        });
    });

    group.bench_function("reverse_sort", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", "-r", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls -r");
            black_box(output);
        });
    });

    group.finish();
}

/// Benchmark memory efficiency
fn bench_memory_usage(c: &mut Criterion) {
    let temp_dir = create_test_directory(10000); // Large directory
    let path = temp_dir.path().to_str().unwrap();

    c.bench_function("large_directory_memory", |b| {
        b.iter(|| {
            let output = Command::new("cargo")
                .args(&["run", "--release", "--", "--stats", path])
                .current_dir(".")
                .output()
                .expect("Failed to run ls with stats");
            black_box(output);
        });
    });
}

criterion_group!(
    benches,
    bench_basic_listing,
    bench_long_format,
    bench_recursive_listing,
    bench_json_output,
    bench_windows_attributes,
    bench_parallel_workers,
    bench_sorting,
    bench_memory_usage
);

criterion_main!(benches);
