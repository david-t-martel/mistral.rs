use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::memory::MemoryStats;
use crate::platforms::Platform;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResults {
    pub timestamp: DateTime<Utc>,
    pub platform: Platform,
    pub is_baseline: bool,
    pub utilities: Vec<UtilityResult>,
    pub summary: BenchmarkSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityResult {
    pub name: String,
    pub test_cases: Vec<TestCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    pub name: String,
    pub winutils_result: ExecutionResult,
    pub native_result: Option<ExecutionResult>,
    pub memory_stats: Option<MemoryStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub iterations: u32,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub total_utilities: usize,
    pub total_test_cases: usize,
    pub successful_tests: usize,
    pub failed_tests: usize,
    pub average_speedup: f64,
    pub total_duration: Duration,
    pub performance_score: f64, // 0.0 to 1.0
    pub memory_efficiency_score: f64,
    pub regression_flags: Vec<RegressionFlag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionFlag {
    pub utility: String,
    pub test_case: String,
    pub issue_type: RegressionType,
    pub severity: Severity,
    pub description: String,
    pub impact_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionType {
    Performance,
    Memory,
    Correctness,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub baseline_results: BenchmarkResults,
    pub current_results: BenchmarkResults,
    pub regressions: Vec<RegressionFlag>,
    pub improvements: Vec<ImprovementFlag>,
    pub overall_score_change: f64,
    pub summary: ComparisonSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementFlag {
    pub utility: String,
    pub test_case: String,
    pub improvement_type: ImprovementType,
    pub description: String,
    pub improvement_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementType {
    Performance,
    Memory,
    Stability,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonSummary {
    pub performance_change_percent: f64,
    pub memory_change_percent: f64,
    pub stability_change_percent: f64,
    pub overall_verdict: Verdict,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Verdict {
    Passed,
    Failed,
    Warning,
}

impl BenchmarkResults {
    pub fn new(timestamp: DateTime<Utc>, platform: Platform, is_baseline: bool) -> Self {
        Self {
            timestamp,
            platform,
            is_baseline,
            utilities: Vec::new(),
            summary: BenchmarkSummary::default(),
        }
    }

    pub fn add_utility_result(&mut self, result: UtilityResult) {
        self.utilities.push(result);
        self.update_summary();
    }

    fn update_summary(&mut self) {
        let total_utilities = self.utilities.len();
        let total_test_cases: usize = self.utilities.iter()
            .map(|u| u.test_cases.len())
            .sum();

        let successful_tests = self.utilities.iter()
            .flat_map(|u| &u.test_cases)
            .filter(|tc| tc.winutils_result.success)
            .count();

        let failed_tests = total_test_cases - successful_tests;

        // Calculate average speedup vs native implementations
        let mut speedup_sum = 0.0;
        let mut speedup_count = 0;

        for utility in &self.utilities {
            for test_case in &utility.test_cases {
                if let Some(native_result) = &test_case.native_result {
                    if native_result.success && test_case.winutils_result.success {
                        let speedup = native_result.duration.as_nanos() as f64
                            / test_case.winutils_result.duration.as_nanos() as f64;
                        speedup_sum += speedup;
                        speedup_count += 1;
                    }
                }
            }
        }

        let average_speedup = if speedup_count > 0 {
            speedup_sum / speedup_count as f64
        } else {
            1.0
        };

        // Calculate total duration
        let total_duration = self.utilities.iter()
            .flat_map(|u| &u.test_cases)
            .map(|tc| tc.winutils_result.duration)
            .sum();

        // Calculate performance score (0.0 to 1.0)
        let performance_score = if total_test_cases > 0 {
            (successful_tests as f64 / total_test_cases as f64) *
            (average_speedup / (average_speedup + 1.0)) // Normalize speedup
        } else {
            0.0
        };

        // Calculate memory efficiency score
        let memory_efficiency_score = self.utilities.iter()
            .flat_map(|u| &u.test_cases)
            .filter_map(|tc| tc.memory_stats.as_ref())
            .map(|ms| ms.memory_efficiency_score)
            .sum::<f64>() /
            self.utilities.iter()
                .flat_map(|u| &u.test_cases)
                .filter(|tc| tc.memory_stats.is_some())
                .count() as f64;

        self.summary = BenchmarkSummary {
            total_utilities,
            total_test_cases,
            successful_tests,
            failed_tests,
            average_speedup,
            total_duration,
            performance_score,
            memory_efficiency_score: if memory_efficiency_score.is_finite() { memory_efficiency_score } else { 1.0 },
            regression_flags: Vec::new(), // Will be populated during comparison
        };
    }

    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let results: Self = serde_json::from_str(&content)?;
        Ok(results)
    }

    pub fn get_utility_result(&self, name: &str) -> Option<&UtilityResult> {
        self.utilities.iter().find(|u| u.name == name)
    }

    pub fn get_test_case_result(&self, utility: &str, test_case: &str) -> Option<&TestCase> {
        self.get_utility_result(utility)?
            .test_cases.iter()
            .find(|tc| tc.name == test_case)
    }
}

impl Default for BenchmarkSummary {
    fn default() -> Self {
        Self {
            total_utilities: 0,
            total_test_cases: 0,
            successful_tests: 0,
            failed_tests: 0,
            average_speedup: 1.0,
            total_duration: Duration::from_secs(0),
            performance_score: 0.0,
            memory_efficiency_score: 1.0,
            regression_flags: Vec::new(),
        }
    }
}

pub fn compare_results(baseline: &BenchmarkResults, current: &BenchmarkResults, threshold: f64) -> Result<ComparisonResult> {
    let mut regressions = Vec::new();
    let mut improvements = Vec::new();

    // Compare each utility and test case
    for current_utility in &current.utilities {
        if let Some(baseline_utility) = baseline.get_utility_result(&current_utility.name) {
            for current_test in &current_utility.test_cases {
                if let Some(baseline_test) = baseline_utility.test_cases.iter()
                    .find(|tc| tc.name == current_test.name) {

                    // Performance comparison
                    let performance_change = calculate_performance_change(
                        &baseline_test.winutils_result,
                        &current_test.winutils_result
                    );

                    if performance_change < -threshold {
                        regressions.push(RegressionFlag {
                            utility: current_utility.name.clone(),
                            test_case: current_test.name.clone(),
                            issue_type: RegressionType::Performance,
                            severity: classify_severity(performance_change.abs()),
                            description: format!(
                                "Performance regression: {:.1}% slower",
                                performance_change.abs()
                            ),
                            impact_percent: performance_change.abs(),
                        });
                    } else if performance_change > threshold {
                        improvements.push(ImprovementFlag {
                            utility: current_utility.name.clone(),
                            test_case: current_test.name.clone(),
                            improvement_type: ImprovementType::Performance,
                            description: format!(
                                "Performance improvement: {:.1}% faster",
                                performance_change
                            ),
                            improvement_percent: performance_change,
                        });
                    }

                    // Memory comparison
                    if let (Some(baseline_memory), Some(current_memory)) =
                        (&baseline_test.memory_stats, &current_test.memory_stats) {

                        let memory_change = calculate_memory_change(baseline_memory, current_memory);

                        if memory_change > threshold {
                            regressions.push(RegressionFlag {
                                utility: current_utility.name.clone(),
                                test_case: current_test.name.clone(),
                                issue_type: RegressionType::Memory,
                                severity: classify_severity(memory_change),
                                description: format!(
                                    "Memory usage increased: {:.1}%",
                                    memory_change
                                ),
                                impact_percent: memory_change,
                            });
                        } else if memory_change < -threshold {
                            improvements.push(ImprovementFlag {
                                utility: current_utility.name.clone(),
                                test_case: current_test.name.clone(),
                                improvement_type: ImprovementType::Memory,
                                description: format!(
                                    "Memory usage decreased: {:.1}%",
                                    memory_change.abs()
                                ),
                                improvement_percent: memory_change.abs(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Calculate overall scores
    let baseline_score = baseline.summary.performance_score;
    let current_score = current.summary.performance_score;
    let overall_score_change = ((current_score - baseline_score) / baseline_score) * 100.0;

    let performance_change_percent =
        ((current.summary.average_speedup - baseline.summary.average_speedup) /
         baseline.summary.average_speedup) * 100.0;

    let memory_change_percent =
        ((current.summary.memory_efficiency_score - baseline.summary.memory_efficiency_score) /
         baseline.summary.memory_efficiency_score) * 100.0;

    let stability_change_percent =
        (((current.summary.successful_tests as f64 / current.summary.total_test_cases as f64) -
          (baseline.summary.successful_tests as f64 / baseline.summary.total_test_cases as f64)) /
         (baseline.summary.successful_tests as f64 / baseline.summary.total_test_cases as f64)) * 100.0;

    let overall_verdict = if regressions.iter().any(|r| matches!(r.severity, Severity::Critical)) {
        Verdict::Failed
    } else if !regressions.is_empty() {
        Verdict::Warning
    } else {
        Verdict::Passed
    };

    let summary = ComparisonSummary {
        performance_change_percent,
        memory_change_percent,
        stability_change_percent,
        overall_verdict,
    };

    Ok(ComparisonResult {
        baseline_results: baseline.clone(),
        current_results: current.clone(),
        regressions,
        improvements,
        overall_score_change,
        summary,
    })
}

fn calculate_performance_change(baseline: &ExecutionResult, current: &ExecutionResult) -> f64 {
    if !baseline.success || !current.success {
        return 0.0;
    }

    let baseline_time = baseline.duration.as_nanos() as f64;
    let current_time = current.duration.as_nanos() as f64;

    ((baseline_time - current_time) / baseline_time) * 100.0
}

fn calculate_memory_change(baseline: &MemoryStats, current: &MemoryStats) -> f64 {
    if baseline.peak_memory_kb == 0 {
        return 0.0;
    }

    ((current.peak_memory_kb as f64 - baseline.peak_memory_kb as f64) /
     baseline.peak_memory_kb as f64) * 100.0
}

fn classify_severity(impact_percent: f64) -> Severity {
    if impact_percent >= 20.0 {
        Severity::Critical
    } else if impact_percent >= 10.0 {
        Severity::High
    } else if impact_percent >= 5.0 {
        Severity::Medium
    } else {
        Severity::Low
    }
}

impl ComparisonResult {
    pub fn has_regressions(&self) -> bool {
        !self.regressions.is_empty()
    }

    pub fn has_critical_regressions(&self) -> bool {
        self.regressions.iter().any(|r| matches!(r.severity, Severity::Critical))
    }

    pub fn print_regressions(&self) {
        use colored::*;

        for regression in &self.regressions {
            let severity_color = match regression.severity {
                Severity::Critical => "red",
                Severity::High => "yellow",
                Severity::Medium => "blue",
                Severity::Low => "white",
            };

            println!("{}: {} - {} ({})",
                format!("{:?}", regression.severity).color(severity_color).bold(),
                regression.utility,
                regression.test_case,
                regression.description
            );
        }
    }

    pub fn print_summary(&self) {
        use colored::*;

        println!("\n{}", "📊 Benchmark Comparison Summary".bright_blue().bold());
        println!("Performance change: {:.1}%", self.summary.performance_change_percent);
        println!("Memory efficiency change: {:.1}%", self.summary.memory_change_percent);
        println!("Stability change: {:.1}%", self.summary.stability_change_percent);

        if !self.improvements.is_empty() {
            println!("\n{}", "✅ Improvements detected:".bright_green());
            for improvement in &self.improvements {
                println!("  {} - {}: {}",
                    improvement.utility,
                    improvement.test_case,
                    improvement.description
                );
            }
        }

        let verdict_text = match self.summary.overall_verdict {
            Verdict::Passed => "PASSED".bright_green(),
            Verdict::Warning => "WARNING".bright_yellow(),
            Verdict::Failed => "FAILED".bright_red(),
        };

        println!("\nOverall verdict: {}", verdict_text.bold());
    }
}
