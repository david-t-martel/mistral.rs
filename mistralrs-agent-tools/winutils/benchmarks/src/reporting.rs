use anyhow::Result;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::metrics::{BenchmarkResults, ComparisonResult, UtilityResult, TestCase, Severity};

pub struct ReportGenerator {
    template_engine: Option<Box<dyn TemplateEngine>>,
}

pub trait TemplateEngine {
    fn render_html(&self, template: &str, data: &serde_json::Value) -> Result<String>;
}

impl ReportGenerator {
    pub fn new() -> Self {
        Self {
            template_engine: None,
        }
    }

    pub fn generate_html_report(&self, results: &BenchmarkResults, output_path: &Path) -> Result<()> {
        let html_content = self.generate_html_content(results)?;

        // Ensure parent directory exists
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, html_content)?;

        // Copy CSS and JS assets
        self.copy_web_assets(output_path.parent().unwrap())?;

        Ok(())
    }

    pub fn generate_markdown_report(&self, results: &BenchmarkResults, output_path: &Path) -> Result<()> {
        let markdown_content = self.generate_markdown_content(results)?;

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, markdown_content)?;
        Ok(())
    }

    pub fn generate_comparison_report(&self, comparison: &ComparisonResult, output_path: &Path) -> Result<()> {
        let html_content = self.generate_comparison_html(comparison)?;

        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(output_path, html_content)?;
        self.copy_web_assets(output_path.parent().unwrap())?;

        Ok(())
    }

    fn generate_html_content(&self, results: &BenchmarkResults) -> Result<String> {
        let html = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WinUtils Benchmark Report</title>
    <link rel="stylesheet" href="styles.css">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <script src="https://cdn.jsdelivr.net/npm/plotly.js-dist@latest/plotly.min.js"></script>
</head>
<body>
    <div class="container">
        <header class="report-header">
            <h1>🚀 WinUtils Performance Benchmark Report</h1>
            <div class="report-meta">
                <span class="timestamp">Generated: {}</span>
                <span class="platform">Platform: {} {}</span>
                <span class="baseline">{}</span>
            </div>
        </header>

        <section class="summary-section">
            <h2>📊 Executive Summary</h2>
            <div class="summary-grid">
                <div class="summary-card">
                    <h3>Utilities Tested</h3>
                    <div class="metric-value">{}</div>
                </div>
                <div class="summary-card">
                    <h3>Test Cases</h3>
                    <div class="metric-value">{}</div>
                </div>
                <div class="summary-card success">
                    <h3>Success Rate</h3>
                    <div class="metric-value">{:.1}%</div>
                </div>
                <div class="summary-card performance">
                    <h3>Avg Speedup</h3>
                    <div class="metric-value">{:.1}x</div>
                </div>
            </div>
        </section>

        <section class="performance-section">
            <h2>⚡ Performance Overview</h2>
            <div class="chart-container">
                <canvas id="performanceChart" width="800" height="400"></canvas>
            </div>
        </section>

        <section class="memory-section">
            <h2>🧠 Memory Analysis</h2>
            <div class="chart-container">
                <div id="memoryChart" style="width:100%;height:400px;"></div>
            </div>
        </section>

        <section class="utilities-section">
            <h2>🔧 Individual Utility Results</h2>
            <div class="utilities-grid">
                {}
            </div>
        </section>

        <section class="details-section">
            <h2>📋 Detailed Results</h2>
            <div class="details-content">
                {}
            </div>
        </section>
    </div>

    <script>
        {}
    </script>
</body>
</html>
        "#,
            results.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            results.platform.os,
            results.platform.arch,
            if results.is_baseline { "Baseline Run" } else { "Comparison Run" },
            results.summary.total_utilities,
            results.summary.total_test_cases,
            (results.summary.successful_tests as f64 / results.summary.total_test_cases as f64) * 100.0,
            results.summary.average_speedup,
            self.generate_utilities_html(&results.utilities)?,
            self.generate_detailed_results_html(&results.utilities)?,
            self.generate_javascript_charts(results)?
        );

        Ok(html)
    }

    fn generate_utilities_html(&self, utilities: &[UtilityResult]) -> Result<String> {
        let mut html = String::new();

        for utility in utilities {
            let success_rate = utility.test_cases.iter()
                .filter(|tc| tc.winutils_result.success)
                .count() as f64 / utility.test_cases.len() as f64 * 100.0;

            let avg_duration_ms = utility.test_cases.iter()
                .map(|tc| tc.winutils_result.duration.as_millis())
                .sum::<u128>() as f64 / utility.test_cases.len() as f64;

            let speedup = utility.test_cases.iter()
                .filter_map(|tc| {
                    tc.native_result.as_ref().map(|nr| {
                        nr.duration.as_nanos() as f64 / tc.winutils_result.duration.as_nanos() as f64
                    })
                })
                .sum::<f64>() / utility.test_cases.iter()
                .filter(|tc| tc.native_result.is_some())
                .count().max(1) as f64;

            html.push_str(&format!(r#"
                <div class="utility-card">
                    <h3>{}</h3>
                    <div class="utility-metrics">
                        <span class="metric">Success: {:.1}%</span>
                        <span class="metric">Avg Time: {:.1}ms</span>
                        <span class="metric">Speedup: {:.1}x</span>
                    </div>
                    <div class="test-cases">
                        {}
                    </div>
                </div>
            "#,
                utility.name,
                success_rate,
                avg_duration_ms,
                speedup,
                utility.test_cases.iter()
                    .map(|tc| format!(
                        r#"<span class="test-case {}">{}</span>"#,
                        if tc.winutils_result.success { "success" } else { "failure" },
                        tc.name
                    ))
                    .collect::<Vec<_>>()
                    .join("")
            ));
        }

        Ok(html)
    }

    fn generate_detailed_results_html(&self, utilities: &[UtilityResult]) -> Result<String> {
        let mut html = String::new();

        for utility in utilities {
            html.push_str(&format!(r#"
                <div class="utility-details">
                    <h3>{}</h3>
                    <table class="results-table">
                        <thead>
                            <tr>
                                <th>Test Case</th>
                                <th>WinUtils Time</th>
                                <th>Native Time</th>
                                <th>Speedup</th>
                                <th>Memory Peak</th>
                                <th>Status</th>
                            </tr>
                        </thead>
                        <tbody>
                            {}
                        </tbody>
                    </table>
                </div>
            "#,
                utility.name,
                utility.test_cases.iter()
                    .map(|tc| self.generate_test_case_row(tc))
                    .collect::<Result<Vec<_>>>()?
                    .join("")
            ));
        }

        Ok(html)
    }

    fn generate_test_case_row(&self, test_case: &TestCase) -> Result<String> {
        let winutils_time_ms = test_case.winutils_result.duration.as_millis();
        let native_time_str = test_case.native_result.as_ref()
            .map(|nr| format!("{}ms", nr.duration.as_millis()))
            .unwrap_or_else(|| "N/A".to_string());

        let speedup_str = test_case.native_result.as_ref()
            .map(|nr| format!("{:.1}x",
                nr.duration.as_nanos() as f64 / test_case.winutils_result.duration.as_nanos() as f64))
            .unwrap_or_else(|| "N/A".to_string());

        let memory_str = test_case.memory_stats.as_ref()
            .map(|ms| format!("{}KB", ms.peak_memory_kb))
            .unwrap_or_else(|| "N/A".to_string());

        let status_class = if test_case.winutils_result.success { "success" } else { "failure" };
        let status_text = if test_case.winutils_result.success { "✅ Pass" } else { "❌ Fail" };

        Ok(format!(r#"
            <tr class="{}">
                <td>{}</td>
                <td>{}ms</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>
        "#,
            status_class,
            test_case.name,
            winutils_time_ms,
            native_time_str,
            speedup_str,
            memory_str,
            status_text
        ))
    }

    fn generate_javascript_charts(&self, results: &BenchmarkResults) -> Result<String> {
        // Prepare data for charts
        let utility_names: Vec<String> = results.utilities.iter()
            .map(|u| u.name.clone())
            .collect();

        let performance_data: Vec<f64> = results.utilities.iter()
            .map(|u| {
                let avg_time = u.test_cases.iter()
                    .map(|tc| tc.winutils_result.duration.as_millis() as f64)
                    .sum::<f64>() / u.test_cases.len() as f64;
                avg_time
            })
            .collect();

        let speedup_data: Vec<f64> = results.utilities.iter()
            .map(|u| {
                let speedups: Vec<f64> = u.test_cases.iter()
                    .filter_map(|tc| {
                        tc.native_result.as_ref().map(|nr| {
                            nr.duration.as_nanos() as f64 / tc.winutils_result.duration.as_nanos() as f64
                        })
                    })
                    .collect();

                if speedups.is_empty() { 1.0 } else { speedups.iter().sum::<f64>() / speedups.len() as f64 }
            })
            .collect();

        let memory_data: Vec<f64> = results.utilities.iter()
            .map(|u| {
                let avg_memory = u.test_cases.iter()
                    .filter_map(|tc| tc.memory_stats.as_ref())
                    .map(|ms| ms.peak_memory_kb as f64)
                    .sum::<f64>() / u.test_cases.iter()
                    .filter(|tc| tc.memory_stats.is_some())
                    .count().max(1) as f64;
                avg_memory
            })
            .collect();

        Ok(format!(r#"
            // Performance Chart
            const performanceCtx = document.getElementById('performanceChart').getContext('2d');
            new Chart(performanceCtx, {{
                type: 'bar',
                data: {{
                    labels: {},
                    datasets: [{{
                        label: 'Execution Time (ms)',
                        data: {},
                        backgroundColor: 'rgba(54, 162, 235, 0.6)',
                        borderColor: 'rgba(54, 162, 235, 1)',
                        borderWidth: 1
                    }}, {{
                        label: 'Speedup vs Native',
                        data: {},
                        backgroundColor: 'rgba(255, 99, 132, 0.6)',
                        borderColor: 'rgba(255, 99, 132, 1)',
                        borderWidth: 1,
                        yAxisID: 'y1'
                    }}]
                }},
                options: {{
                    responsive: true,
                    scales: {{
                        y: {{
                            type: 'linear',
                            display: true,
                            position: 'left',
                            title: {{ display: true, text: 'Time (ms)' }}
                        }},
                        y1: {{
                            type: 'linear',
                            display: true,
                            position: 'right',
                            title: {{ display: true, text: 'Speedup Factor' }},
                            grid: {{ drawOnChartArea: false }}
                        }}
                    }}
                }}
            }});

            // Memory Chart
            const memoryTrace = {{
                x: {},
                y: {},
                type: 'scatter',
                mode: 'markers+lines',
                name: 'Peak Memory Usage',
                marker: {{ size: 8, color: 'rgba(75, 192, 192, 0.6)' }}
            }};

            const memoryLayout = {{
                title: 'Memory Usage by Utility',
                xaxis: {{ title: 'Utility' }},
                yaxis: {{ title: 'Memory (KB)' }},
                showlegend: false
            }};

            Plotly.newPlot('memoryChart', [memoryTrace], memoryLayout);
        "#,
            serde_json::to_string(&utility_names)?,
            serde_json::to_string(&performance_data)?,
            serde_json::to_string(&speedup_data)?,
            serde_json::to_string(&utility_names)?,
            serde_json::to_string(&memory_data)?
        ))
    }

    fn generate_markdown_content(&self, results: &BenchmarkResults) -> Result<String> {
        let mut markdown = String::new();

        markdown.push_str(&format!(r#"# WinUtils Benchmark Report

Generated: {}
Platform: {} {}
Type: {}

## Executive Summary

- **Utilities Tested**: {}
- **Test Cases**: {}
- **Success Rate**: {:.1}%
- **Average Speedup**: {:.1}x
- **Performance Score**: {:.3}
- **Memory Efficiency**: {:.3}

"#,
            results.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
            results.platform.os,
            results.platform.arch,
            if results.is_baseline { "Baseline Run" } else { "Comparison Run" },
            results.summary.total_utilities,
            results.summary.total_test_cases,
            (results.summary.successful_tests as f64 / results.summary.total_test_cases as f64) * 100.0,
            results.summary.average_speedup,
            results.summary.performance_score,
            results.summary.memory_efficiency_score
        ));

        markdown.push_str("## Utility Results\n\n");

        for utility in &results.utilities {
            markdown.push_str(&format!("### {}\n\n", utility.name));

            markdown.push_str("| Test Case | WinUtils Time | Native Time | Speedup | Memory Peak | Status |\n");
            markdown.push_str("|-----------|---------------|-------------|---------|-------------|--------|\n");

            for test_case in &utility.test_cases {
                let winutils_time_ms = test_case.winutils_result.duration.as_millis();
                let native_time_str = test_case.native_result.as_ref()
                    .map(|nr| format!("{}ms", nr.duration.as_millis()))
                    .unwrap_or_else(|| "N/A".to_string());

                let speedup_str = test_case.native_result.as_ref()
                    .map(|nr| format!("{:.1}x",
                        nr.duration.as_nanos() as f64 / test_case.winutils_result.duration.as_nanos() as f64))
                    .unwrap_or_else(|| "N/A".to_string());

                let memory_str = test_case.memory_stats.as_ref()
                    .map(|ms| format!("{}KB", ms.peak_memory_kb))
                    .unwrap_or_else(|| "N/A".to_string());

                let status = if test_case.winutils_result.success { "✅ Pass" } else { "❌ Fail" };

                markdown.push_str(&format!(
                    "| {} | {}ms | {} | {} | {} | {} |\n",
                    test_case.name,
                    winutils_time_ms,
                    native_time_str,
                    speedup_str,
                    memory_str,
                    status
                ));
            }

            markdown.push_str("\n");
        }

        if !results.summary.regression_flags.is_empty() {
            markdown.push_str("## Issues Detected\n\n");

            for flag in &results.summary.regression_flags {
                let severity_emoji = match flag.severity {
                    Severity::Critical => "🔴",
                    Severity::High => "🟡",
                    Severity::Medium => "🔵",
                    Severity::Low => "⚪",
                };

                markdown.push_str(&format!(
                    "- {} **{}** in `{}` - `{}`: {} ({:.1}% impact)\n",
                    severity_emoji,
                    format!("{:?}", flag.issue_type),
                    flag.utility,
                    flag.test_case,
                    flag.description,
                    flag.impact_percent
                ));
            }
        }

        Ok(markdown)
    }

    fn generate_comparison_html(&self, comparison: &ComparisonResult) -> Result<String> {
        // Implementation for comparison report HTML
        let html = format!(r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WinUtils Benchmark Comparison</title>
    <link rel="stylesheet" href="styles.css">
</head>
<body>
    <div class="container">
        <header class="report-header">
            <h1>📊 WinUtils Benchmark Comparison</h1>
            <div class="comparison-meta">
                <div class="baseline-info">
                    <h3>Baseline</h3>
                    <span>{}</span>
                </div>
                <div class="current-info">
                    <h3>Current</h3>
                    <span>{}</span>
                </div>
            </div>
        </header>

        <section class="verdict-section">
            <h2>🎯 Overall Verdict</h2>
            <div class="verdict {}">
                {}
            </div>
            <div class="summary-changes">
                <div class="change-item">
                    <span class="label">Performance:</span>
                    <span class="value {}">{:+.1}%</span>
                </div>
                <div class="change-item">
                    <span class="label">Memory:</span>
                    <span class="value {}">{:+.1}%</span>
                </div>
                <div class="change-item">
                    <span class="label">Stability:</span>
                    <span class="value {}">{:+.1}%</span>
                </div>
            </div>
        </section>

        {}

        {}
    </div>
</body>
</html>
        "#,
            comparison.baseline_results.timestamp.format("%Y-%m-%d %H:%M:%S"),
            comparison.current_results.timestamp.format("%Y-%m-%d %H:%M:%S"),
            match comparison.summary.overall_verdict {
                crate::metrics::Verdict::Passed => "passed",
                crate::metrics::Verdict::Warning => "warning",
                crate::metrics::Verdict::Failed => "failed",
            },
            match comparison.summary.overall_verdict {
                crate::metrics::Verdict::Passed => "✅ PASSED",
                crate::metrics::Verdict::Warning => "⚠️ WARNING",
                crate::metrics::Verdict::Failed => "❌ FAILED",
            },
            if comparison.summary.performance_change_percent >= 0.0 { "positive" } else { "negative" },
            comparison.summary.performance_change_percent,
            if comparison.summary.memory_change_percent <= 0.0 { "positive" } else { "negative" },
            comparison.summary.memory_change_percent,
            if comparison.summary.stability_change_percent >= 0.0 { "positive" } else { "negative" },
            comparison.summary.stability_change_percent,
            self.generate_regressions_html(&comparison.regressions)?,
            self.generate_improvements_html(&comparison.improvements)?
        );

        Ok(html)
    }

    fn generate_regressions_html(&self, regressions: &[crate::metrics::RegressionFlag]) -> Result<String> {
        if regressions.is_empty() {
            return Ok("<section class=\"no-regressions\"><h2>✅ No Regressions Detected</h2></section>".to_string());
        }

        let mut html = String::from("<section class=\"regressions-section\"><h2>⚠️ Regressions Detected</h2><div class=\"regressions-list\">");

        for regression in regressions {
            let severity_class = match regression.severity {
                Severity::Critical => "critical",
                Severity::High => "high",
                Severity::Medium => "medium",
                Severity::Low => "low",
            };

            html.push_str(&format!(r#"
                <div class="regression-item {}">
                    <div class="regression-header">
                        <span class="utility">{}</span>
                        <span class="test-case">{}</span>
                        <span class="severity">{:?}</span>
                    </div>
                    <div class="regression-details">
                        <span class="description">{}</span>
                        <span class="impact">{:.1}% impact</span>
                    </div>
                </div>
            "#, severity_class, regression.utility, regression.test_case, regression.severity, regression.description, regression.impact_percent));
        }

        html.push_str("</div></section>");
        Ok(html)
    }

    fn generate_improvements_html(&self, improvements: &[crate::metrics::ImprovementFlag]) -> Result<String> {
        if improvements.is_empty() {
            return Ok(String::new());
        }

        let mut html = String::from("<section class=\"improvements-section\"><h2>🚀 Improvements Detected</h2><div class=\"improvements-list\">");

        for improvement in improvements {
            html.push_str(&format!(r#"
                <div class="improvement-item">
                    <div class="improvement-header">
                        <span class="utility">{}</span>
                        <span class="test-case">{}</span>
                    </div>
                    <div class="improvement-details">
                        <span class="description">{}</span>
                        <span class="improvement">{:.1}% improvement</span>
                    </div>
                </div>
            "#, improvement.utility, improvement.test_case, improvement.description, improvement.improvement_percent));
        }

        html.push_str("</div></section>");
        Ok(html)
    }

    fn copy_web_assets(&self, output_dir: &Path) -> Result<()> {
        // Create CSS file
        let css_content = include_str!("../assets/styles.css");
        fs::write(output_dir.join("styles.css"), css_content)?;

        Ok(())
    }
}

impl Default for ReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}
