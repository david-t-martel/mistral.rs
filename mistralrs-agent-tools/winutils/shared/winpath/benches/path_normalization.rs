//! Benchmarks for path normalization performance.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use winpath::{normalize_path, normalize_path_cow, PathNormalizer};

fn bench_path_formats(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_formats");

    let test_paths = vec![
        ("dos_short", r"C:\Users\David"),
        ("dos_forward", "C:/Users/David/Documents"),
        ("wsl_path", "/mnt/c/users/david/documents"),
        ("cygwin_path", "/cygdrive/c/users/david/documents"),
        ("unc_path", r"\\?\C:\Users\David\Documents"),
        ("unix_like", "//c/users/david/documents"),
        ("mixed_separators", r"C:\Users/David\Documents/file.txt"),
        ("long_path", &format!("C:\\{}", "very_long_directory_name".repeat(20))),
        ("relative_path", r"Documents\Projects\..\Projects\file.txt"),
    ];

    for (name, path) in test_paths {
        group.bench_with_input(BenchmarkId::new("normalize_path", name), &path, |b, path| {
            b.iter(|| normalize_path(black_box(path)));
        });

        group.bench_with_input(BenchmarkId::new("normalize_path_cow", name), &path, |b, path| {
            b.iter(|| normalize_path_cow(black_box(path)));
        });
    }

    group.finish();
}

fn bench_cached_normalization(c: &mut Criterion) {
    let mut group = c.benchmark_group("cached_normalization");

    let normalizer = PathNormalizer::new();
    let normalizer_no_cache = PathNormalizer::without_cache();

    let test_paths = vec![
        "/mnt/c/users/david",
        "/mnt/d/projects",
        "C:/Windows/System32",
        "/cygdrive/e/temp",
        r"\\?\C:\Program Files\App",
    ];

    group.bench_function("with_cache", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(normalizer.normalize(path).unwrap());
            }
        });
    });

    group.bench_function("without_cache", |b| {
        b.iter(|| {
            for path in &test_paths {
                black_box(normalizer_no_cache.normalize(path).unwrap());
            }
        });
    });

    group.finish();
}

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    let normalizer = PathNormalizer::new();

    let small_batch: Vec<&str> = vec![
        "/mnt/c/users/david",
        "C:/Windows",
        "/cygdrive/d/temp",
    ];

    let large_batch: Vec<&str> = (0..100)
        .map(|i| match i % 4 {
            0 => "/mnt/c/users/david",
            1 => "C:/Windows/System32",
            2 => "/cygdrive/d/projects",
            _ => r"\\?\C:\Program Files\App",
        })
        .collect();

    group.bench_function("small_batch_individual", |b| {
        b.iter(|| {
            for path in &small_batch {
                black_box(normalizer.normalize(path).unwrap());
            }
        });
    });

    group.bench_function("small_batch_batch", |b| {
        b.iter(|| {
            black_box(normalizer.normalize_batch(&small_batch).unwrap());
        });
    });

    group.bench_function("large_batch_individual", |b| {
        b.iter(|| {
            for path in &large_batch {
                black_box(normalizer.normalize(path).unwrap());
            }
        });
    });

    group.bench_function("large_batch_batch", |b| {
        b.iter(|| {
            black_box(normalizer.normalize_batch(&large_batch).unwrap());
        });
    });

    group.finish();
}

fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    let test_path = "C:/Users/David/Documents/Projects/rust/target/debug/deps/project.exe";

    group.bench_function("split_operations", |b| {
        b.iter(|| {
            let parts: Vec<&str> = black_box(test_path).split('/').collect();
            black_box(parts);
        });
    });

    group.bench_function("replace_operations", |b| {
        b.iter(|| {
            let result = black_box(test_path).replace('/', "\\");
            black_box(result);
        });
    });

    group.bench_function("char_iteration", |b| {
        b.iter(|| {
            let mut count = 0;
            for ch in black_box(test_path).chars() {
                if ch == '/' || ch == '\\' {
                    count += 1;
                }
            }
            black_box(count);
        });
    });

    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    let already_normalized = r"C:\Users\David\Documents";
    let needs_normalization = "/mnt/c/users/david/documents";

    group.bench_function("cow_no_allocation", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(already_normalized)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("cow_with_allocation", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(needs_normalization)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("string_always_allocates", |b| {
        b.iter(|| {
            let result = normalize_path(black_box(already_normalized)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

fn bench_pathological_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("pathological_cases");

    // Very long path
    let very_long_path = format!("/mnt/c/{}", "component/".repeat(100));

    // Path with many dot components
    let many_dots_path = format!("C:\\{}\\file.txt", "folder\\..\\folder\\.".repeat(50));

    // Path with many redundant separators
    let redundant_seps_path = format!("C:{}Users{}David", "\\\\".repeat(20), "\\\\".repeat(20));

    group.bench_function("very_long_path", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(&very_long_path));
            black_box(result);
        });
    });

    group.bench_function("many_dot_components", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(&many_dots_path));
            black_box(result);
        });
    });

    group.bench_function("redundant_separators", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(&redundant_seps_path));
            black_box(result);
        });
    });

    group.finish();
}

#[cfg(feature = "cache")]
fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    let normalizer_small = PathNormalizer::with_config(winpath::NormalizerConfig {
        cache_size: 10,
        ..Default::default()
    });

    let normalizer_large = PathNormalizer::with_config(winpath::NormalizerConfig {
        cache_size: 1000,
        ..Default::default()
    });

    let test_paths: Vec<String> = (0..50)
        .map(|i| format!("/mnt/c/test/path/number/{}", i))
        .collect();

    // Fill caches
    for path in &test_paths {
        let _ = normalizer_small.normalize(path);
        let _ = normalizer_large.normalize(path);
    }

    group.bench_function("small_cache_hit", |b| {
        b.iter(|| {
            let path = &test_paths[black_box(0) % 10]; // Always hit
            black_box(normalizer_small.normalize(path).unwrap());
        });
    });

    group.bench_function("large_cache_hit", |b| {
        b.iter(|| {
            let path = &test_paths[black_box(0) % 50]; // Always hit
            black_box(normalizer_large.normalize(path).unwrap());
        });
    });

    group.bench_function("cache_miss", |b| {
        b.iter(|| {
            let path = format!("/mnt/c/new/path/{}", black_box(1000));
            black_box(normalizer_large.normalize(&path).unwrap());
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_path_formats,
    bench_cached_normalization,
    bench_batch_operations,
    bench_string_operations,
    bench_memory_allocation,
    bench_pathological_cases,
);

#[cfg(feature = "cache")]
criterion_group!(cache_benches, bench_cache_performance);

#[cfg(feature = "cache")]
criterion_main!(benches, cache_benches);

#[cfg(not(feature = "cache"))]
criterion_main!(benches);
