use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::path::PathBuf;

mod benchmarks;
mod config;
mod memory;
mod metrics;
mod platforms;
mod reporting;
mod utils;

use benchmarks::BenchmarkSuite;
use config::BenchmarkConfig;
use reporting::ReportGenerator;

#[cfg(feature = "memory-profiling")]
#[global_allocator]
static ALLOC: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

#[derive(Parser)]
#[command(name = "benchmark-runner")]
#[command(about = "WinUtils Performance Benchmarking Framework")]
#[command(version = "1.0.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run performance benchmarks
    Run {
        /// Configuration file path
        #[arg(short, long, default_value = "benchmarks/config/default.toml")]
        config: PathBuf,

        /// Output directory for results
        #[arg(short, long, default_value = "benchmarks/reports")]
        output: PathBuf,

        /// Filter benchmarks by name pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Enable memory profiling
        #[arg(long)]
        memory_profile: bool,

        /// Compare against native utilities
        #[arg(long)]
        compare_native: bool,

        /// Baseline run for regression detection
        #[arg(long)]
        baseline: bool,
    },

    /// Generate reports from benchmark results
    Report {
        /// Results directory
        #[arg(short, long, default_value = "benchmarks/reports")]
        input: PathBuf,

        /// Output format (html, json, markdown)
        #[arg(short, long, default_value = "html")]
        format: String,

        /// Include performance regression analysis
        #[arg(long)]
        regression_analysis: bool,
    },

    /// Compare two benchmark runs
    Compare {
        /// Baseline results file
        #[arg(short, long)]
        baseline: PathBuf,

        /// Current results file
        #[arg(short, long)]
        current: PathBuf,

        /// Regression threshold percentage
        #[arg(short, long, default_value = "5.0")]
        threshold: f64,
    },

    /// Validate benchmark environment
    Validate,

    /// Generate benchmark configuration
    Config {
        /// Output configuration file
        #[arg(short, long, default_value = "benchmarks/config/generated.toml")]
        output: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            config,
            output,
            filter,
            memory_profile,
            compare_native,
            baseline,
        } => {
            println!("{}", "🚀 Starting WinUtils Benchmark Suite".bright_green().bold());

            let config = BenchmarkConfig::load(&config)
                .with_context(|| format!("Failed to load config from {}", config.display()))?;

            let mut suite = BenchmarkSuite::new(config);

            if let Some(filter) = filter {
                suite.filter_benchmarks(&filter);
            }

            suite.set_memory_profiling(memory_profile);
            suite.set_native_comparison(compare_native);
            suite.set_baseline_mode(baseline);

            let results = suite.run().await
                .context("Failed to run benchmark suite")?;

            // Save results
            std::fs::create_dir_all(&output)?;
            let results_file = output.join("results.json");
            results.save_to_file(&results_file)?;

            println!("{}", format!("✅ Benchmarks completed! Results saved to {}", results_file.display()).bright_green());

            // Generate initial report
            let report_gen = ReportGenerator::new();
            report_gen.generate_html_report(&results, &output.join("report.html"))?;

            Ok(())
        }

        Commands::Report {
            input,
            format,
            regression_analysis,
        } => {
            println!("{}", "📊 Generating benchmark reports".bright_blue().bold());

            let report_gen = ReportGenerator::new();

            match format.as_str() {
                "html" => {
                    let results_file = input.join("results.json");
                    let results = metrics::BenchmarkResults::load_from_file(&results_file)?;

                    let output_file = input.join("report.html");
                    report_gen.generate_html_report(&results, &output_file)?;

                    println!("{}", format!("✅ HTML report generated: {}", output_file.display()).bright_green());
                }
                "json" => {
                    // JSON format is already saved during benchmark run
                    println!("{}", "✅ JSON results already available".bright_green());
                }
                "markdown" => {
                    let results_file = input.join("results.json");
                    let results = metrics::BenchmarkResults::load_from_file(&results_file)?;

                    let output_file = input.join("report.md");
                    report_gen.generate_markdown_report(&results, &output_file)?;

                    println!("{}", format!("✅ Markdown report generated: {}", output_file.display()).bright_green());
                }
                _ => {
                    anyhow::bail!("Unsupported format: {}. Use html, json, or markdown", format);
                }
            }

            Ok(())
        }

        Commands::Compare {
            baseline,
            current,
            threshold,
        } => {
            println!("{}", "🔍 Comparing benchmark results".bright_yellow().bold());

            let baseline_results = metrics::BenchmarkResults::load_from_file(&baseline)?;
            let current_results = metrics::BenchmarkResults::load_from_file(&current)?;

            let comparison = metrics::compare_results(&baseline_results, &current_results, threshold)?;

            if comparison.has_regressions() {
                println!("{}", "❌ Performance regressions detected!".bright_red().bold());
                comparison.print_regressions();
                std::process::exit(1);
            } else {
                println!("{}", "✅ No significant performance regressions detected".bright_green());
                comparison.print_summary();
            }

            Ok(())
        }

        Commands::Validate => {
            println!("{}", "🔧 Validating benchmark environment".bright_blue().bold());

            utils::validate_environment().await?;

            println!("{}", "✅ Environment validation passed".bright_green());
            Ok(())
        }

        Commands::Config { output } => {
            println!("{}", "⚙️  Generating benchmark configuration".bright_blue().bold());

            let config = BenchmarkConfig::generate_default();
            config.save_to_file(&output)?;

            println!("{}", format!("✅ Configuration generated: {}", output.display()).bright_green());
            Ok(())
        }
    }
}
