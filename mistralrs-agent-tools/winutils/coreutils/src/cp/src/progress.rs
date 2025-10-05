//! Progress reporting for copy operations
//!
//! This module provides progress reporting functionality for large file copy operations,
//! including progress bars, transfer rate calculation, and ETA estimation.

use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::thread;
use indicatif::{ProgressBar, ProgressStyle, HumanBytes, HumanDuration};

/// Progress reporter for copy operations
pub struct ProgressReporter {
    show_progress: bool,
    verbose: bool,
    progress_bars: Arc<Mutex<Vec<FileProgress>>>,
}

/// Progress tracking for individual files
struct FileProgress {
    progress_bar: ProgressBar,
    start_time: Instant,
    bytes_copied: AtomicU64,
    total_bytes: AtomicU64,
    completed: AtomicBool,
}

impl ProgressReporter {
    /// Create a new progress reporter
    pub fn new(show_progress: bool, verbose: bool) -> Self {
        Self {
            show_progress,
            verbose,
            progress_bars: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Check if progress should be shown
    pub fn should_show_progress(&self) -> bool {
        self.show_progress
    }

    /// Start progress tracking for a file
    pub fn start_file_progress(&self, source_path: &str, total_bytes: u64) -> Option<usize> {
        if !self.show_progress {
            return None;
        }

        let progress_bar = ProgressBar::new(total_bytes);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}, ETA {eta})")
                .unwrap()
                .progress_chars("##-"),
        );

        progress_bar.set_message(format!("Copying {}", source_path));

        let file_progress = FileProgress {
            progress_bar,
            start_time: Instant::now(),
            bytes_copied: AtomicU64::new(0),
            total_bytes: AtomicU64::new(total_bytes),
            completed: AtomicBool::new(false),
        };

        let mut progress_bars = self.progress_bars.lock().unwrap();
        progress_bars.push(file_progress);
        Some(progress_bars.len() - 1)
    }

    /// Update progress for a file
    pub fn update_progress(&self, bytes_copied: u64, total_bytes: u64) {
        if !self.show_progress {
            return;
        }

        let progress_bars = self.progress_bars.lock().unwrap();
        if let Some(file_progress) = progress_bars.last() {
            file_progress.bytes_copied.store(bytes_copied, Ordering::Relaxed);
            file_progress.total_bytes.store(total_bytes, Ordering::Relaxed);
            file_progress.progress_bar.set_position(bytes_copied);
            file_progress.progress_bar.set_length(total_bytes);
        }
    }

    /// Update progress for a specific file index
    pub fn update_file_progress(&self, file_index: usize, bytes_copied: u64, total_bytes: u64) {
        if !self.show_progress {
            return;
        }

        let progress_bars = self.progress_bars.lock().unwrap();
        if let Some(file_progress) = progress_bars.get(file_index) {
            file_progress.bytes_copied.store(bytes_copied, Ordering::Relaxed);
            file_progress.total_bytes.store(total_bytes, Ordering::Relaxed);
            file_progress.progress_bar.set_position(bytes_copied);
            file_progress.progress_bar.set_length(total_bytes);
        }
    }

    /// Complete progress tracking for a file
    pub fn complete_file_progress(&self, file_index: Option<usize>) {
        if !self.show_progress {
            return;
        }

        let progress_bars = self.progress_bars.lock().unwrap();

        if let Some(index) = file_index {
            if let Some(file_progress) = progress_bars.get(index) {
                file_progress.completed.store(true, Ordering::Relaxed);
                let total = file_progress.total_bytes.load(Ordering::Relaxed);
                file_progress.progress_bar.set_position(total);
                file_progress.progress_bar.finish_with_message("Completed");
            }
        } else if let Some(file_progress) = progress_bars.last() {
            file_progress.completed.store(true, Ordering::Relaxed);
            let total = file_progress.total_bytes.load(Ordering::Relaxed);
            file_progress.progress_bar.set_position(total);
            file_progress.progress_bar.finish_with_message("Completed");
        }
    }

    /// Start a summary progress bar for multiple files
    pub fn start_summary_progress(&self, total_files: usize, total_bytes: u64) -> Option<SummaryProgress> {
        if !self.show_progress || total_files <= 1 {
            return None;
        }

        let progress_bar = ProgressBar::new(total_bytes);
        progress_bar.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({bytes_per_sec}) Files: {pos}/{len}")
                .unwrap()
                .progress_chars("##-"),
        );

        progress_bar.set_message("Copying files...");

        Some(SummaryProgress {
            progress_bar,
            start_time: Instant::now(),
            total_files,
            completed_files: AtomicU64::new(0),
            total_bytes: AtomicU64::new(total_bytes),
            copied_bytes: AtomicU64::new(0),
        })
    }

    /// Print verbose message
    pub fn verbose_message(&self, message: &str) {
        if self.verbose {
            println!("{}", message);
        }
    }

    /// Print transfer statistics
    pub fn print_transfer_stats(&self, bytes_transferred: u64, duration: Duration) {
        if !self.verbose && !self.show_progress {
            return;
        }

        let rate = if duration.as_secs() > 0 {
            bytes_transferred / duration.as_secs()
        } else {
            bytes_transferred
        };

        println!(
            "Transferred {} in {} ({}/s)",
            HumanBytes(bytes_transferred),
            HumanDuration(duration),
            HumanBytes(rate)
        );
    }

    /// Calculate and display performance metrics
    pub fn display_performance_metrics(&self,
        total_bytes: u64,
        total_duration: Duration,
        parallel_threads: Option<usize>
    ) {
        if !self.verbose {
            return;
        }

        let throughput = if total_duration.as_secs() > 0 {
            total_bytes as f64 / total_duration.as_secs_f64()
        } else {
            total_bytes as f64
        };

        println!("\nPerformance Metrics:");
        println!("  Total data copied: {}", HumanBytes(total_bytes));
        println!("  Total time: {}", HumanDuration(total_duration));
        println!("  Average throughput: {}/s", HumanBytes(throughput as u64));

        if let Some(threads) = parallel_threads {
            println!("  Parallel threads used: {}", threads);
            let per_thread_throughput = throughput / threads as f64;
            println!("  Per-thread throughput: {}/s", HumanBytes(per_thread_throughput as u64));
        }

        // Calculate efficiency metrics
        let efficiency = calculate_copy_efficiency(total_bytes, total_duration);
        println!("  Copy efficiency: {:.1}%", efficiency * 100.0);
    }
}

/// Summary progress for multiple file operations
pub struct SummaryProgress {
    progress_bar: ProgressBar,
    start_time: Instant,
    total_files: usize,
    completed_files: AtomicU64,
    total_bytes: AtomicU64,
    copied_bytes: AtomicU64,
}

impl SummaryProgress {
    /// Update summary progress
    pub fn update(&self, files_completed: u64, bytes_copied: u64) {
        self.completed_files.store(files_completed, Ordering::Relaxed);
        self.copied_bytes.store(bytes_copied, Ordering::Relaxed);

        self.progress_bar.set_position(bytes_copied);
        self.progress_bar.set_length(self.total_bytes.load(Ordering::Relaxed));

        // Update message with file count
        let message = format!(
            "Copying files... ({}/{})",
            files_completed,
            self.total_files
        );
        self.progress_bar.set_message(message);
    }

    /// Complete summary progress
    pub fn complete(&self) {
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        self.progress_bar.set_position(total_bytes);
        self.progress_bar.finish_with_message(format!(
            "Completed {} files",
            self.total_files
        ));
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

/// Calculate copy efficiency based on theoretical vs actual performance
fn calculate_copy_efficiency(bytes_copied: u64, duration: Duration) -> f64 {
    // Theoretical maximum for modern SSDs: ~500 MB/s
    // Theoretical maximum for HDDs: ~150 MB/s
    // We'll use a conservative estimate of 100 MB/s as baseline
    const BASELINE_THROUGHPUT: f64 = 100.0 * 1024.0 * 1024.0; // 100 MB/s

    let actual_throughput = if duration.as_secs_f64() > 0.0 {
        bytes_copied as f64 / duration.as_secs_f64()
    } else {
        bytes_copied as f64
    };

    (actual_throughput / BASELINE_THROUGHPUT).min(1.0)
}

/// Progress callback trait for external progress reporting
pub trait ProgressCallback: Send + Sync {
    fn on_progress(&self, bytes_copied: u64, total_bytes: u64);
    fn on_complete(&self);
    fn on_error(&self, error: &str);
}

/// Multi-threaded progress aggregator
pub struct ProgressAggregator {
    reporters: Vec<Arc<ProgressReporter>>,
    summary: Option<SummaryProgress>,
}

impl ProgressAggregator {
    /// Create a new progress aggregator
    pub fn new() -> Self {
        Self {
            reporters: Vec::new(),
            summary: None,
        }
    }

    /// Add a progress reporter
    pub fn add_reporter(&mut self, reporter: Arc<ProgressReporter>) {
        self.reporters.push(reporter);
    }

    /// Start aggregated progress tracking
    pub fn start_summary(&mut self, total_files: usize, total_bytes: u64) {
        if let Some(reporter) = self.reporters.first() {
            self.summary = reporter.start_summary_progress(total_files, total_bytes);
        }
    }

    /// Update aggregated progress
    pub fn update_summary(&self, files_completed: u64, bytes_copied: u64) {
        if let Some(ref summary) = self.summary {
            summary.update(files_completed, bytes_copied);
        }
    }

    /// Complete aggregated progress
    pub fn complete_summary(&self) {
        if let Some(ref summary) = self.summary {
            summary.complete();
        }
    }
}

impl Default for ProgressAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_reporter_creation() {
        let reporter = ProgressReporter::new(true, false);
        assert!(reporter.should_show_progress());
        assert!(!reporter.verbose);
    }

    #[test]
    fn test_calculate_copy_efficiency() {
        // Perfect efficiency case (100 MB/s)
        let efficiency = calculate_copy_efficiency(100 * 1024 * 1024, Duration::from_secs(1));
        assert!((efficiency - 1.0).abs() < 0.01);

        // Half efficiency case (50 MB/s)
        let efficiency = calculate_copy_efficiency(50 * 1024 * 1024, Duration::from_secs(1));
        assert!((efficiency - 0.5).abs() < 0.01);

        // Zero duration case
        let efficiency = calculate_copy_efficiency(1024 * 1024, Duration::from_secs(0));
        assert!(efficiency > 0.0);
    }

    #[test]
    fn test_progress_aggregator() {
        let mut aggregator = ProgressAggregator::new();
        let reporter = Arc::new(ProgressReporter::new(false, false));

        aggregator.add_reporter(reporter);
        aggregator.start_summary(5, 1024 * 1024);
        aggregator.update_summary(2, 512 * 1024);
        aggregator.complete_summary();

        // Test passes if no panics occur
    }

    #[test]
    fn test_summary_progress_lifecycle() {
        let reporter = ProgressReporter::new(true, true);

        if let Some(summary) = reporter.start_summary_progress(5, 1024 * 1024) {
            summary.update(2, 512 * 1024);
            assert!(summary.elapsed() >= Duration::from_nanos(1));
            summary.complete();
        }

        // Test passes if no panics occur
    }
}
