use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use sysinfo::{System, SystemExt, Pid, PidExt, ProcessExt};

use crate::config::{UtilityConfig, TestCase};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryStats {
    pub peak_memory_kb: u64,
    pub average_memory_kb: u64,
    pub memory_samples: Vec<MemorySample>,
    pub allocation_count: u64,
    pub deallocation_count: u64,
    pub peak_heap_kb: u64,
    pub total_allocations_kb: u64,
    pub memory_efficiency_score: f64, // 0.0 to 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySample {
    pub timestamp_ms: u64,
    pub memory_kb: u64,
    pub heap_kb: u64,
    pub stack_kb: u64,
}

pub struct MemoryProfiler {
    sampling_interval: Duration,
    system: System,
}

impl MemoryProfiler {
    pub fn new() -> Self {
        Self {
            sampling_interval: Duration::from_millis(10), // Sample every 10ms
            system: System::new_all(),
        }
    }

    pub fn set_sampling_interval(&mut self, interval: Duration) {
        self.sampling_interval = interval;
    }

    #[cfg(feature = "memory-profiling")]
    pub async fn profile_command(&self, utility: &UtilityConfig, test_case: &TestCase, working_dir: &Path) -> Result<MemoryStats> {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let binary_path = working_dir.parent()
            .unwrap()
            .join("target/release")
            .join(&format!("{}.exe", utility.name));

        let samples = Arc::new(Mutex::new(Vec::new()));
        let samples_clone = Arc::clone(&samples);

        // Start the command
        let mut cmd = Command::new(&binary_path);
        cmd.args(&test_case.args);
        cmd.current_dir(working_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let start_time = Instant::now();
        let mut child = cmd.spawn()?;
        let pid = child.id();

        // Start memory monitoring in a separate thread
        let sampling_interval = self.sampling_interval;
        let monitor_handle = thread::spawn(move || {
            let mut system = System::new();

            while let Ok(None) = child.try_wait() {
                system.refresh_process(Pid::from_u32(pid));

                if let Some(process) = system.process(Pid::from_u32(pid)) {
                    let memory_kb = process.memory() / 1024;
                    let virtual_memory_kb = process.virtual_memory() / 1024;

                    let sample = MemorySample {
                        timestamp_ms: start_time.elapsed().as_millis() as u64,
                        memory_kb,
                        heap_kb: memory_kb, // Approximation
                        stack_kb: virtual_memory_kb.saturating_sub(memory_kb),
                    };

                    if let Ok(mut samples) = samples_clone.lock() {
                        samples.push(sample);
                    }
                }

                thread::sleep(sampling_interval);
            }
        });

        // Wait for command to complete
        let output = child.wait_with_output()?;

        // Wait for monitoring to complete
        let _ = monitor_handle.join();

        // Analyze memory usage
        let samples = samples.lock().unwrap().clone();
        self.analyze_memory_usage(samples, output.status.success())
    }

    #[cfg(not(feature = "memory-profiling"))]
    pub async fn profile_command(&self, _utility: &UtilityConfig, _test_case: &TestCase, _working_dir: &Path) -> Result<MemoryStats> {
        Ok(MemoryStats::default())
    }

    fn analyze_memory_usage(&self, samples: Vec<MemorySample>, success: bool) -> Result<MemoryStats> {
        if samples.is_empty() {
            return Ok(MemoryStats::default());
        }

        let peak_memory_kb = samples.iter().map(|s| s.memory_kb).max().unwrap_or(0);
        let average_memory_kb = samples.iter().map(|s| s.memory_kb).sum::<u64>() / samples.len() as u64;
        let peak_heap_kb = samples.iter().map(|s| s.heap_kb).max().unwrap_or(0);

        // Calculate memory efficiency score
        let memory_efficiency_score = if peak_memory_kb > 0 {
            (average_memory_kb as f64 / peak_memory_kb as f64).min(1.0)
        } else {
            1.0
        };

        // Estimate allocations (simplified heuristic)
        let mut allocation_count = 0u64;
        let mut prev_memory = 0u64;

        for sample in &samples {
            if sample.memory_kb > prev_memory {
                allocation_count += 1;
            }
            prev_memory = sample.memory_kb;
        }

        let deallocation_count = allocation_count.saturating_sub(1);
        let total_allocations_kb = samples.iter().map(|s| s.memory_kb).sum::<u64>();

        Ok(MemoryStats {
            peak_memory_kb,
            average_memory_kb,
            memory_samples: samples,
            allocation_count,
            deallocation_count,
            peak_heap_kb,
            total_allocations_kb,
            memory_efficiency_score,
        })
    }

    /// Profile memory usage with jemalloc statistics
    #[cfg(feature = "memory-profiling")]
    pub fn get_jemalloc_stats() -> Result<JemallocStats> {
        use jemalloc_ctl::{stats, epoch};

        // Update statistics
        epoch::advance()?;

        let allocated = stats::allocated::read()?;
        let active = stats::active::read()?;
        let mapped = stats::mapped::read()?;
        let resident = stats::resident::read()?;

        Ok(JemallocStats {
            allocated_bytes: allocated,
            active_bytes: active,
            mapped_bytes: mapped,
            resident_bytes: resident,
        })
    }

    #[cfg(not(feature = "memory-profiling"))]
    pub fn get_jemalloc_stats() -> Result<JemallocStats> {
        Ok(JemallocStats::default())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JemallocStats {
    pub allocated_bytes: u64,
    pub active_bytes: u64,
    pub mapped_bytes: u64,
    pub resident_bytes: u64,
}

impl MemoryStats {
    pub fn is_within_limits(&self, max_memory_kb: u64) -> bool {
        self.peak_memory_kb <= max_memory_kb
    }

    pub fn has_memory_leaks(&self, threshold_kb: u64) -> bool {
        if self.memory_samples.len() < 2 {
            return false;
        }

        let start_memory = self.memory_samples[0].memory_kb;
        let end_memory = self.memory_samples.last().unwrap().memory_kb;

        end_memory.saturating_sub(start_memory) > threshold_kb
    }

    pub fn memory_growth_rate(&self) -> f64 {
        if self.memory_samples.len() < 2 {
            return 0.0;
        }

        let start = &self.memory_samples[0];
        let end = self.memory_samples.last().unwrap();

        if end.timestamp_ms == start.timestamp_ms {
            return 0.0;
        }

        let time_diff_seconds = (end.timestamp_ms - start.timestamp_ms) as f64 / 1000.0;
        let memory_diff_kb = end.memory_kb as f64 - start.memory_kb as f64;

        memory_diff_kb / time_diff_seconds // KB per second
    }

    pub fn generate_summary(&self) -> String {
        format!(
            "Peak: {}KB, Avg: {}KB, Efficiency: {:.2}, Growth Rate: {:.2}KB/s",
            self.peak_memory_kb,
            self.average_memory_kb,
            self.memory_efficiency_score,
            self.memory_growth_rate()
        )
    }
}

/// Windows-specific memory profiling using native APIs
#[cfg(windows)]
pub mod windows {
    use super::*;
    use windows::Win32::System::ProcessStatus::*;
    use windows::Win32::System::Threading::*;
    use windows::Win32::Foundation::*;

    pub struct WindowsMemoryProfiler {
        process_handle: HANDLE,
    }

    impl WindowsMemoryProfiler {
        pub fn new(process_id: u32) -> Result<Self> {
            unsafe {
                let handle = OpenProcess(
                    PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                    false,
                    process_id
                )?;

                Ok(Self {
                    process_handle: handle,
                })
            }
        }

        pub fn get_memory_info(&self) -> Result<PROCESS_MEMORY_COUNTERS_EX> {
            unsafe {
                let mut pmc = std::mem::zeroed::<PROCESS_MEMORY_COUNTERS_EX>();
                pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;

                GetProcessMemoryInfo(
                    self.process_handle,
                    &mut pmc as *mut _ as *mut PROCESS_MEMORY_COUNTERS,
                    pmc.cb
                )?;

                Ok(pmc)
            }
        }

        pub fn get_working_set_size(&self) -> Result<usize> {
            let info = self.get_memory_info()?;
            Ok(info.WorkingSetSize)
        }

        pub fn get_peak_working_set_size(&self) -> Result<usize> {
            let info = self.get_memory_info()?;
            Ok(info.PeakWorkingSetSize)
        }

        pub fn get_private_bytes(&self) -> Result<usize> {
            let info = self.get_memory_info()?;
            Ok(info.PrivateUsage)
        }
    }

    impl Drop for WindowsMemoryProfiler {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.process_handle);
            }
        }
    }
}
