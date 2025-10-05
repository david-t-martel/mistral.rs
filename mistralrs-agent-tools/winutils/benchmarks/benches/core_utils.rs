use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::fs;
use std::path::Path;
use std::time::Duration;

// Mock implementations for demonstration - replace with actual utility functions
mod utils {
    pub fn count_lines(data: &[u8]) -> usize {
        memchr::memchr_iter(b'\n', data).count()
    }

    pub fn count_words(data: &[u8]) -> usize {
        data.split(|&b| b == b' ' || b == b'\n' || b == b'\t')
            .filter(|w| !w.is_empty())
            .count()
    }

    pub fn sort_lines(lines: &mut Vec<String>) {
        lines.sort_unstable();
    }

    pub fn parallel_sort_lines(lines: &mut Vec<String>) {
        use rayon::prelude::*;
        lines.par_sort_unstable();
    }

    pub fn hash_blake3(data: &[u8]) -> blake3::Hash {
        blake3::hash(data)
    }

    pub fn hash_sha256(data: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }
}

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    let line = b"The quick brown fox jumps over the lazy dog\n";
    while data.len() < size {
        data.extend_from_slice(line);
    }
    data.truncate(size);
    data
}

fn generate_random_lines(count: usize) -> Vec<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| {
            format!(
                "line_{:08x}_{}",
                rng.gen::<u32>(),
                "x".repeat(rng.gen_range(10..100))
            )
        })
        .collect()
}

fn benchmark_wc(c: &mut Criterion) {
    let mut group = c.benchmark_group("wc");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for size in [1024, 1024 * 1024, 10 * 1024 * 1024] {
        let data = generate_test_data(size);

        group.bench_with_input(
            BenchmarkId::new("lines", format_size(size)),
            &data,
            |b, data| b.iter(|| utils::count_lines(black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("words", format_size(size)),
            &data,
            |b, data| b.iter(|| utils::count_words(black_box(data))),
        );
    }
    group.finish();
}

fn benchmark_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("sort");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for size in [1000, 10000, 50000] {
        let data = generate_random_lines(size);

        group.bench_function(BenchmarkId::new("sequential", size), |b| {
            b.iter_batched(
                || data.clone(),
                |mut lines| utils::sort_lines(black_box(&mut lines)),
                criterion::BatchSize::LargeInput,
            )
        });

        group.bench_function(BenchmarkId::new("parallel", size), |b| {
            b.iter_batched(
                || data.clone(),
                |mut lines| utils::parallel_sort_lines(black_box(&mut lines)),
                criterion::BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

fn benchmark_hash(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash");
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(5));

    for size in [1024, 1024 * 1024, 10 * 1024 * 1024] {
        let data = generate_test_data(size);

        group.bench_with_input(
            BenchmarkId::new("blake3", format_size(size)),
            &data,
            |b, data| b.iter(|| utils::hash_blake3(black_box(data))),
        );

        group.bench_with_input(
            BenchmarkId::new("sha256", format_size(size)),
            &data,
            |b, data| b.iter(|| utils::hash_sha256(black_box(data))),
        );
    }
    group.finish();
}

fn benchmark_path_normalization(c: &mut Criterion) {
    use winpath::normalize_path;

    let test_paths = vec![
        "/mnt/c/Windows/System32",
        "C:\\Windows\\System32",
        "/cygdrive/c/Users/david/Documents",
        "\\\\?\\C:\\Program Files\\Application",
        "../relative/path/to/file.txt",
    ];

    let mut group = c.benchmark_group("path_normalization");

    for path in &test_paths {
        group.bench_with_input(
            BenchmarkId::from_parameter(path),
            path,
            |b, path| b.iter(|| normalize_path(black_box(path))),
        );
    }

    // Benchmark cache performance
    group.bench_function("cached_paths", |b| {
        let paths: Vec<_> = (0..1000).map(|i| format!("/mnt/c/path/{}", i)).collect();
        b.iter(|| {
            for path in &paths {
                normalize_path(black_box(path));
            }
        });
    });

    group.finish();
}

fn format_size(size: usize) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{}KB", size / 1024)
    } else {
        format!("{}MB", size / (1024 * 1024))
    }
}

// Profile with flamegraph support
#[cfg(feature = "flamegraph")]
mod profiling {
    use criterion::profiler::Profiler;
    use pprof::{ProfilerGuard, Report};
    use std::fs::File;
    use std::os::raw::c_int;

    pub struct FlamegraphProfiler<'a> {
        frequency: c_int,
        active_profiler: Option<ProfilerGuard<'a>>,
    }

    impl<'a> FlamegraphProfiler<'a> {
        pub fn new(frequency: c_int) -> Self {
            FlamegraphProfiler {
                frequency,
                active_profiler: None,
            }
        }
    }

    impl<'a> Profiler for FlamegraphProfiler<'a> {
        fn start_profiling(&mut self, _benchmark_id: &str, _benchmark_dir: &Path) {
            self.active_profiler = Some(
                pprof::ProfilerGuardBuilder::default()
                    .frequency(self.frequency)
                    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
                    .build()
                    .unwrap(),
            );
        }

        fn stop_profiling(&mut self, benchmark_id: &str, benchmark_dir: &Path) {
            if let Some(profiler) = self.active_profiler.take() {
                let report = profiler.report().build().unwrap();
                let file =
                    File::create(benchmark_dir.join(format!("{}.svg", benchmark_id))).unwrap();
                report.flamegraph(file).unwrap();
            }
        }
    }
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(100)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(10));
    targets = benchmark_wc, benchmark_sort, benchmark_hash, benchmark_path_normalization
}

criterion_main!(benches);
