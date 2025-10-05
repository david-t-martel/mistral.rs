pub mod benchmarks;
pub mod config;
pub mod memory;
pub mod metrics;
pub mod platforms;
pub mod reporting;
pub mod utils;

pub use benchmarks::BenchmarkSuite;
pub use config::BenchmarkConfig;
pub use memory::{MemoryProfiler, MemoryStats};
pub use metrics::{BenchmarkResults, ComparisonResult};
pub use platforms::{Platform, get_current_platform};
pub use reporting::ReportGenerator;
pub use utils::{validate_environment, format_duration, format_memory};
