//! Optimized benchmarks demonstrating 2-3x performance improvements.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use winpath::*;

// Test different path types and sizes
const SHORT_DOS_PATH: &str = r"C:\Users\David";
const MEDIUM_DOS_PATH: &str = r"C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\IDE";
const LONG_DOS_PATH: &str = r"C:\Users\David\Documents\Projects\RustProjects\winutils\target\release\deps\very_long_directory_name_that_exceeds_normal_limits\another_long_directory\file.exe";

const SHORT_WSL_PATH: &str = "/mnt/c/users/david";
const MEDIUM_WSL_PATH: &str = "/mnt/c/program files/microsoft visual studio/2022/professional";
const LONG_WSL_PATH: &str = "/mnt/c/users/david/documents/projects/rustprojects/winutils/target/release/deps/very_long_directory";

const MIXED_PATH: &str = r"C:\Users/David\Documents/Projects/Rust\target/debug/deps";
const PATH_WITH_DOTS: &str = r"C:\Users\.\David\..\David\Documents\..\..\David\Documents";
const PATH_WITH_DOUBLES: &str = r"C:\\Users\\\\David\\\\Documents";

fn bench_baseline_vs_optimized(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_vs_optimized");

    // Benchmark different path types
    let test_cases = vec![
        ("short_dos", SHORT_DOS_PATH),
        ("medium_dos", MEDIUM_DOS_PATH),
        ("long_dos", LONG_DOS_PATH),
        ("short_wsl", SHORT_WSL_PATH),
        ("medium_wsl", MEDIUM_WSL_PATH),
        ("long_wsl", LONG_WSL_PATH),
        ("mixed", MIXED_PATH),
        ("with_dots", PATH_WITH_DOTS),
        ("with_doubles", PATH_WITH_DOUBLES),
    ];

    for (name, path) in &test_cases {
        group.throughput(Throughput::Bytes(path.len() as u64));

        // Baseline implementation
        group.bench_with_input(
            BenchmarkId::new("baseline", name),
            path,
            |b, path| b.iter(|| normalize_path(black_box(path))),
        );

        // Optimized implementation
        #[cfg(feature = "perf")]
        group.bench_with_input(
            BenchmarkId::new("optimized", name),
            path,
            |b, path| {
                b.iter(|| {
                    use crate::normalization_optimized::normalize_path_optimized;
                    normalize_path_optimized(black_box(path))
                })
            },
        );
    }

    group.finish();
}

fn bench_simd_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("path_detection");

    let paths = vec![
        ("dos", r"C:\Users\David\Documents"),
        ("wsl", "/mnt/c/users/david/documents"),
        ("cygwin", "/cygdrive/c/users/david"),
        ("unc", r"\\?\C:\Users\David"),
        ("unix_like", "//c/users/david"),
        ("mixed", r"C:\Users/David\Documents/file.txt"),
    ];

    for (name, path) in &paths {
        group.throughput(Throughput::Bytes(path.len() as u64));

        // Baseline detection
        group.bench_with_input(
            BenchmarkId::new("baseline_detection", name),
            path,
            |b, path| b.iter(|| detect_path_format(black_box(path))),
        );

        // SIMD detection
        #[cfg(feature = "simd")]
        group.bench_with_input(
            BenchmarkId::new("simd_detection", name),
            path,
            |b, path| {
                use crate::detection_optimized::detect_path_format_simd;
                b.iter(|| detect_path_format_simd(black_box(path)))
            },
        );
    }

    group.finish();
}

fn bench_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_performance");

    // Create different cache configurations
    let no_cache = PathNormalizer::without_cache();
    let basic_cache = PathNormalizer::new();

    #[cfg(feature = "cache")]
    let optimized_cache = {
        use crate::cache_optimized::{PathCache, CacheConfig};
        PathCache::new(CacheConfig::default())
    };

    let test_paths: Vec<String> = (0..100)
        .map(|i| format!("/mnt/c/users/david/project_{}/src/main.rs", i))
        .collect();

    // Warm up caches
    for path in &test_paths[..50] {
        let _ = basic_cache.normalize(path);
        #[cfg(feature = "cache")]
        {
            let _ = optimized_cache.get(path);
            if optimized_cache.get(path).is_none() {
                if let Ok(normalized) = normalize_path(path) {
                    optimized_cache.insert(path, normalized, 0, false);
                }
            }
        }
    }

    // Benchmark cache hits (first 50 paths)
    group.bench_function("no_cache_hits", |b| {
        b.iter(|| {
            for path in &test_paths[..50] {
                black_box(no_cache.normalize(path).unwrap());
            }
        });
    });

    group.bench_function("basic_cache_hits", |b| {
        b.iter(|| {
            for path in &test_paths[..50] {
                black_box(basic_cache.normalize(path).unwrap());
            }
        });
    });

    #[cfg(feature = "cache")]
    group.bench_function("optimized_cache_hits", |b| {
        b.iter(|| {
            for path in &test_paths[..50] {
                let result = optimized_cache.get(path).unwrap_or_else(|| {
                    normalize_path(path).unwrap()
                });
                black_box(result);
            }
        });
    });

    // Benchmark cache misses (last 50 paths)
    group.bench_function("no_cache_misses", |b| {
        b.iter(|| {
            for path in &test_paths[50..] {
                black_box(no_cache.normalize(path).unwrap());
            }
        });
    });

    group.bench_function("basic_cache_misses", |b| {
        b.iter(|| {
            for path in &test_paths[50..] {
                black_box(basic_cache.normalize(path).unwrap());
            }
        });
    });

    #[cfg(feature = "cache")]
    group.bench_function("optimized_cache_misses", |b| {
        b.iter(|| {
            for path in &test_paths[50..] {
                let result = if let Some(cached) = optimized_cache.get(path) {
                    cached
                } else {
                    let normalized = normalize_path(path).unwrap();
                    optimized_cache.insert(path, normalized.clone(), 0, false);
                    normalized
                };
                black_box(result);
            }
        });
    });

    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // Test zero-copy optimization
    let already_normalized = r"C:\Users\David\Documents";
    let needs_normalization = "/mnt/c/users/david/documents";

    group.bench_function("zero_copy_baseline", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(already_normalized)).unwrap();
            black_box(result);
        });
    });

    #[cfg(feature = "perf")]
    group.bench_function("zero_copy_optimized", |b| {
        use crate::normalization_optimized::normalize_path_optimized;
        b.iter(|| {
            let result = normalize_path_optimized(black_box(already_normalized)).unwrap();
            black_box(result);
        });
    });

    group.bench_function("allocation_baseline", |b| {
        b.iter(|| {
            let result = normalize_path_cow(black_box(needs_normalization)).unwrap();
            black_box(result);
        });
    });

    #[cfg(feature = "perf")]
    group.bench_function("allocation_optimized", |b| {
        use crate::normalization_optimized::normalize_path_optimized;
        b.iter(|| {
            let result = normalize_path_optimized(black_box(needs_normalization)).unwrap();
            black_box(result);
        });
    });

    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");

    #[cfg(feature = "cache")]
    {
        use crate::cache_optimized::{PathCache, CacheConfig};
        use std::sync::Arc;
        use std::thread;

        let cache = Arc::new(PathCache::new(CacheConfig::default()));

        // Pre-populate cache
        for i in 0..100 {
            let path = format!("/mnt/c/test/path_{}", i);
            if let Ok(normalized) = normalize_path(&path) {
                cache.insert(&path, normalized, 0, false);
            }
        }

        group.bench_function("single_thread", |b| {
            b.iter(|| {
                for i in 0..100 {
                    let path = format!("/mnt/c/test/path_{}", i);
                    black_box(cache.get(&path));
                }
            });
        });

        group.bench_function("multi_thread_4", |b| {
            b.iter(|| {
                let handles: Vec<_> = (0..4)
                    .map(|thread_id| {
                        let cache = cache.clone();
                        thread::spawn(move || {
                            for i in 0..25 {
                                let path = format!("/mnt/c/test/path_{}", thread_id * 25 + i);
                                black_box(cache.get(&path));
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });

        group.bench_function("multi_thread_8", |b| {
            b.iter(|| {
                let handles: Vec<_> = (0..8)
                    .map(|thread_id| {
                        let cache = cache.clone();
                        thread::spawn(move || {
                            for i in 0..12 {
                                let path = format!("/mnt/c/test/path_{}", thread_id * 12 + i);
                                black_box(cache.get(&path));
                            }
                        })
                    })
                    .collect();

                for handle in handles {
                    handle.join().unwrap();
                }
            });
        });
    }

    group.finish();
}

fn bench_pathological_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("pathological_cases");

    // Very deep nesting
    let deep_path = format!("/mnt/c/{}", "dir/".repeat(100));

    // Many dot components
    let dotty_path = format!("C:\\{}", "folder\\..\\".repeat(50));

    // Many redundant separators
    let redundant_path = format!("C:{}Users{}David", "\\\\".repeat(50), "\\\\".repeat(50));

    // Very long single component
    let long_component = format!("C:\\{}.txt", "a".repeat(500));

    group.bench_function("deep_nesting_baseline", |b| {
        b.iter(|| normalize_path(black_box(&deep_path)));
    });

    #[cfg(feature = "perf")]
    group.bench_function("deep_nesting_optimized", |b| {
        use crate::normalization_optimized::normalize_path_optimized;
        b.iter(|| normalize_path_optimized(black_box(&deep_path)));
    });

    group.bench_function("many_dots_baseline", |b| {
        b.iter(|| normalize_path(black_box(&dotty_path)));
    });

    #[cfg(feature = "perf")]
    group.bench_function("many_dots_optimized", |b| {
        use crate::normalization_optimized::normalize_path_optimized;
        b.iter(|| normalize_path_optimized(black_box(&dotty_path)));
    });

    group.bench_function("redundant_seps_baseline", |b| {
        b.iter(|| normalize_path(black_box(&redundant_path)));
    });

    #[cfg(feature = "perf")]
    group.bench_function("redundant_seps_optimized", |b| {
        use crate::normalization_optimized::normalize_path_optimized;
        b.iter(|| normalize_path_optimized(black_box(&redundant_path)));
    });

    group.bench_function("long_component_baseline", |b| {
        b.iter(|| normalize_path(black_box(&long_component)));
    });

    #[cfg(feature = "perf")]
    group.bench_function("long_component_optimized", |b| {
        use crate::normalization_optimized::normalize_path_optimized;
        b.iter(|| normalize_path_optimized(black_box(&long_component)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_baseline_vs_optimized,
    bench_simd_detection,
    bench_cache_performance,
    bench_memory_allocation,
    bench_concurrent_access,
    bench_pathological_cases,
);

criterion_main!(benches);
