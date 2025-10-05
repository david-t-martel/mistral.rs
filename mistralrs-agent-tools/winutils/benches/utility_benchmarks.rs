// Criterion benchmarks for critical WinUtils utilities
// Measures performance of frequently-used utilities to detect regressions

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use std::process::Command;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;
use std::path::PathBuf;

// Test data generation
fn generate_test_file(size: usize) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_data.txt");
    let mut file = File::create(&file_path).unwrap();

    let line = "The quick brown fox jumps over the lazy dog. ";
    let content = line.repeat(size / line.len());
    file.write_all(content.as_bytes()).unwrap();

    (temp_dir, file_path)
}

fn generate_directory_tree(depth: usize, width: usize) -> TempDir {
    let temp_dir = TempDir::new().unwrap();

    fn create_tree(dir: &std::path::Path, depth: usize, width: usize) {
        if depth == 0 {
            return;
        }

        for i in 0..width {
            let file_path = dir.join(format!("file_{}.txt", i));
            File::create(&file_path).unwrap();

            if depth > 1 {
                let subdir = dir.join(format!("dir_{}", i));
                fs::create_dir(&subdir).unwrap();
                create_tree(&subdir, depth - 1, width / 2);
            }
        }
    }

    create_tree(temp_dir.path(), depth, width);
    temp_dir
}

// Benchmark functions for core utilities

fn bench_cat(c: &mut Criterion) {
    let mut group = c.benchmark_group("cat");

    for size in [1024, 1024 * 100, 1024 * 1024].iter() {
        let (_temp_dir, file_path) = generate_test_file(*size);
        let file_str = file_path.to_str().unwrap();

        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/cat.exe")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_wc(c: &mut Criterion) {
    let mut group = c.benchmark_group("wc");

    for size in [1024, 1024 * 100, 1024 * 1024].iter() {
        let (_temp_dir, file_path) = generate_test_file(*size);
        let file_str = file_path.to_str().unwrap();

        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark line counting
        group.bench_with_input(BenchmarkId::new("lines", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/wc.exe")
                    .arg("-l")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark word counting
        group.bench_with_input(BenchmarkId::new("words", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/wc.exe")
                    .arg("-w")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark byte counting
        group.bench_with_input(BenchmarkId::new("bytes", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/wc.exe")
                    .arg("-c")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_ls(c: &mut Criterion) {
    let mut group = c.benchmark_group("ls");

    for (depth, width) in [(2, 10), (3, 10), (4, 5)].iter() {
        let temp_dir = generate_directory_tree(*depth, *width);
        let dir_str = temp_dir.path().to_str().unwrap();
        let param_name = format!("depth_{}_width_{}", depth, width);

        // Benchmark simple listing
        group.bench_function(BenchmarkId::new("simple", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/ls.exe")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark long format
        group.bench_function(BenchmarkId::new("long", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/ls.exe")
                    .arg("-la")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark recursive listing
        group.bench_function(BenchmarkId::new("recursive", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/ls.exe")
                    .arg("-R")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");

    for size in [100, 1000, 10000].iter() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("numbers.txt");
        let mut file = File::create(&file_path).unwrap();

        // Generate random numbers
        use rand::Rng;
        let mut rng = rand::thread_rng();
        for _ in 0..*size {
            writeln!(file, "{}", rng.gen::<u32>()).unwrap();
        }

        let file_str = file_path.to_str().unwrap();

        // Benchmark text sort
        group.bench_with_input(BenchmarkId::new("text", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/sort.exe")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark numeric sort
        group.bench_with_input(BenchmarkId::new("numeric", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/sort.exe")
                    .arg("-n")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark unique sort
        group.bench_with_input(BenchmarkId::new("unique", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/sort.exe")
                    .arg("-u")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_hashsum(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashsum");

    for size in [1024, 1024 * 100, 1024 * 1024].iter() {
        let (_temp_dir, file_path) = generate_test_file(*size);
        let file_str = file_path.to_str().unwrap();

        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark MD5
        group.bench_with_input(BenchmarkId::new("md5", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/hashsum.exe")
                    .arg("--md5")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark SHA256
        group.bench_with_input(BenchmarkId::new("sha256", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/hashsum.exe")
                    .arg("--sha256")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark Blake3
        group.bench_with_input(BenchmarkId::new("blake3", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/hashsum.exe")
                    .arg("--b3sum")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_grep(c: &mut Criterion) {
    let mut group = c.benchmark_group("grep");

    for size in [1024, 1024 * 100, 1024 * 1024].iter() {
        let (_temp_dir, file_path) = generate_test_file(*size);
        let file_str = file_path.to_str().unwrap();

        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark simple pattern
        group.bench_with_input(BenchmarkId::new("simple", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/grep.exe")
                    .arg("fox")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark regex pattern
        group.bench_with_input(BenchmarkId::new("regex", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/grep.exe")
                    .arg("-E")
                    .arg("[a-z]+")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark case-insensitive
        group.bench_with_input(BenchmarkId::new("case_insensitive", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/grep.exe")
                    .arg("-i")
                    .arg("FOX")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_find(c: &mut Criterion) {
    let mut group = c.benchmark_group("find");

    for (depth, width) in [(2, 10), (3, 10), (4, 5)].iter() {
        let temp_dir = generate_directory_tree(*depth, *width);
        let dir_str = temp_dir.path().to_str().unwrap();
        let param_name = format!("depth_{}_width_{}", depth, width);

        // Benchmark name search
        group.bench_function(BenchmarkId::new("name", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/find.exe")
                    .arg(dir_str)
                    .arg("-name")
                    .arg("*.txt")
                    .output()
                    .unwrap()
            });
        });

        // Benchmark type search
        group.bench_function(BenchmarkId::new("type", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/find.exe")
                    .arg(dir_str)
                    .arg("-type")
                    .arg("f")
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_cp(c: &mut Criterion) {
    let mut group = c.benchmark_group("cp");

    for size in [1024, 1024 * 100, 1024 * 1024].iter() {
        let (_temp_dir, file_path) = generate_test_file(*size);
        let file_str = file_path.to_str().unwrap();
        let dest = file_path.with_extension("copy");
        let dest_str = dest.to_str().unwrap();

        group.throughput(Throughput::Bytes(*size as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                // Remove destination if it exists
                let _ = fs::remove_file(&dest);

                Command::new("target/release/cp.exe")
                    .arg(file_str)
                    .arg(dest_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

// Derive utilities benchmarks

fn bench_fd_wrapper(c: &mut Criterion) {
    let mut group = c.benchmark_group("fd-wrapper");

    for (depth, width) in [(2, 10), (3, 10), (4, 5)].iter() {
        let temp_dir = generate_directory_tree(*depth, *width);
        let dir_str = temp_dir.path().to_str().unwrap();
        let param_name = format!("depth_{}_width_{}", depth, width);

        // Benchmark file search
        group.bench_function(BenchmarkId::new("files", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/fd.exe")
                    .arg("file")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark extension search
        group.bench_function(BenchmarkId::new("extension", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/fd.exe")
                    .arg("-e")
                    .arg("txt")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_rg_wrapper(c: &mut Criterion) {
    let mut group = c.benchmark_group("rg-wrapper");

    for size in [1024, 1024 * 100, 1024 * 1024].iter() {
        let (_temp_dir, file_path) = generate_test_file(*size);
        let file_str = file_path.to_str().unwrap();

        group.throughput(Throughput::Bytes(*size as u64));

        // Benchmark pattern search
        group.bench_with_input(BenchmarkId::new("pattern", size), size, |b, _| {
            b.iter(|| {
                Command::new("target/release/rg.exe")
                    .arg("fox")
                    .arg(file_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree");

    for (depth, width) in [(2, 10), (3, 10), (4, 5)].iter() {
        let temp_dir = generate_directory_tree(*depth, *width);
        let dir_str = temp_dir.path().to_str().unwrap();
        let param_name = format!("depth_{}_width_{}", depth, width);

        // Benchmark full tree
        group.bench_function(BenchmarkId::new("full", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/tree.exe")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark directories only
        group.bench_function(BenchmarkId::new("dirs_only", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/tree.exe")
                    .arg("-d")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });

        // Benchmark with depth limit
        group.bench_function(BenchmarkId::new("depth_limit", &param_name), |b| {
            b.iter(|| {
                Command::new("target/release/tree.exe")
                    .arg("-L")
                    .arg("2")
                    .arg(dir_str)
                    .output()
                    .unwrap()
            });
        });
    }

    group.finish();
}

// Main benchmark groups
criterion_group!(
    core_utils,
    bench_cat,
    bench_wc,
    bench_ls,
    bench_sort,
    bench_hashsum,
    bench_grep,
    bench_find,
    bench_cp
);

criterion_group!(
    derive_utils,
    bench_fd_wrapper,
    bench_rg_wrapper,
    bench_tree
);

criterion_main!(core_utils, derive_utils);
