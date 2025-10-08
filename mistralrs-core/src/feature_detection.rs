/// Runtime feature detection and capability reporting for mistral.rs
///
/// This module provides compile-time and runtime checks for optional features
/// to give users better error messages and debugging information.
use std::fmt;

/// Available features that can be enabled at compile time
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSet {
    /// Flash attention support (v1/v2)
    pub flash_attn: bool,
    /// Flash attention v3 support
    pub flash_attn_v3: bool,
    /// CUDA support
    pub cuda: bool,
    /// Metal support (macOS GPU)
    pub metal: bool,
    /// Accelerate framework (macOS CPU)
    pub accelerate: bool,
    /// Intel MKL support
    pub mkl: bool,
    /// Python bindings
    pub pyo3: bool,
}

impl FeatureSet {
    /// Get the current compile-time feature set
    pub fn current() -> Self {
        Self {
            flash_attn: cfg!(feature = "flash-attn"),
            flash_attn_v3: cfg!(feature = "flash-attn-v3"),
            cuda: cfg!(feature = "cuda"),
            metal: cfg!(feature = "metal"),
            accelerate: cfg!(feature = "accelerate"),
            mkl: cfg!(feature = "mkl"),
            pyo3: cfg!(feature = "pyo3_macros"),
        }
    }

    /// Check if any flash attention variant is available
    pub fn has_flash_attn(&self) -> bool {
        self.flash_attn || self.flash_attn_v3
    }

    /// Check if any hardware acceleration is available
    pub fn has_hw_accel(&self) -> bool {
        self.cuda || self.metal
    }

    /// Check if any CPU acceleration is available
    pub fn has_cpu_accel(&self) -> bool {
        self.accelerate || self.mkl
    }

    /// Get a human-readable description of enabled features
    pub fn summary(&self) -> String {
        let mut features = Vec::new();

        if self.flash_attn {
            features.push("flash-attn");
        }
        if self.flash_attn_v3 {
            features.push("flash-attn-v3");
        }
        if self.cuda {
            features.push("cuda");
        }
        if self.metal {
            features.push("metal");
        }
        if self.accelerate {
            features.push("accelerate");
        }
        if self.mkl {
            features.push("mkl");
        }
        if self.pyo3 {
            features.push("pyo3");
        }

        if features.is_empty() {
            "no optional features enabled".to_string()
        } else {
            features.join(", ")
        }
    }

    /// Get compilation suggestions for missing features
    pub fn compilation_hints(&self, needed: &[&str]) -> String {
        let mut hints = Vec::new();

        for feature in needed {
            match *feature {
                "flash-attn" if !self.flash_attn => {
                    hints.push("--features flash-attn".to_string());
                }
                "flash-attn-v3" if !self.flash_attn_v3 => {
                    hints.push("--features flash-attn-v3".to_string());
                }
                "cuda" if !self.cuda => {
                    hints.push("--features cuda".to_string());
                }
                "metal" if !self.metal => {
                    hints.push("--features metal".to_string());
                }
                _ => {}
            }
        }

        if hints.is_empty() {
            String::new()
        } else {
            format!(
                "Recompile mistral.rs with: cargo build --release {}",
                hints.join(" ")
            )
        }
    }
}

impl fmt::Display for FeatureSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "mistral.rs features: {}", self.summary())
    }
}

/// Quantization method capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantSupport {
    /// Fully supported and production-ready
    Stable,
    /// Implemented but experimental - may have issues
    Experimental,
    /// Partially implemented - some operations may fail
    Partial,
    /// Not implemented yet
    Unimplemented,
}

impl QuantSupport {
    pub fn is_usable(&self) -> bool {
        matches!(self, QuantSupport::Stable | QuantSupport::Experimental)
    }

    pub fn warning_message(&self, method: &str) -> Option<String> {
        match self {
            QuantSupport::Experimental => Some(format!(
                "{} quantization is experimental. Consider using GGUF or ISQ for production.",
                method
            )),
            QuantSupport::Partial => Some(format!(
                "{} quantization is partially implemented. Some operations may fail. Use GGUF or ISQ instead.",
                method
            )),
            QuantSupport::Unimplemented => Some(format!(
                "{} quantization is not yet implemented.",
                method
            )),
            QuantSupport::Stable => None,
        }
    }
}

/// Get quantization support level for different methods
pub fn quantization_support(method: &str) -> QuantSupport {
    match method.to_lowercase().as_str() {
        "gguf" | "ggml" | "isq" | "hqq" => QuantSupport::Stable,
        "bnb" | "bitsandbytes" => QuantSupport::Partial,
        "afq" => QuantSupport::Experimental,
        "fp8" | "blockwise_fp8" => QuantSupport::Stable,
        _ => QuantSupport::Unimplemented,
    }
}

/// Check if a feature is available and return helpful error if not
pub fn require_feature(feature: &str) -> Result<(), String> {
    let features = FeatureSet::current();

    let available = match feature {
        "flash-attn" => features.flash_attn,
        "flash-attn-v3" => features.flash_attn_v3,
        "cuda" => features.cuda,
        "metal" => features.metal,
        "accelerate" => features.accelerate,
        "mkl" => features.mkl,
        "pyo3" => features.pyo3,
        _ => return Err(format!("Unknown feature: {}", feature)),
    };

    if available {
        Ok(())
    } else {
        Err(format!(
            "Feature '{}' is not enabled. {}",
            feature,
            features.compilation_hints(&[feature])
        ))
    }
}

/// Print feature detection information at startup
pub fn log_feature_info() {
    let features = FeatureSet::current();
    tracing::info!("{}", features);

    if !features.has_hw_accel() {
        tracing::warn!(
            "No GPU acceleration enabled. Performance may be limited. \
            Consider enabling cuda or metal features for better performance."
        );
    }

    if !features.has_flash_attn() {
        tracing::info!(
            "Flash attention not enabled. Standard attention will be used. \
            For 2-3x faster inference, enable flash-attn or flash-attn-v3."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_detection() {
        let features = FeatureSet::current();
        // At least one of these should be true in most builds
        assert!(
            features.cuda
                || features.metal
                || features.accelerate
                || features.mkl
                || !features.has_hw_accel()
        );
    }

    #[test]
    fn test_quantization_support() {
        assert_eq!(quantization_support("gguf"), QuantSupport::Stable);
        assert_eq!(quantization_support("hqq"), QuantSupport::Stable);
        assert_eq!(quantization_support("bnb"), QuantSupport::Partial);
        assert_eq!(quantization_support("afq"), QuantSupport::Experimental);
        assert_eq!(quantization_support("unknown"), QuantSupport::Unimplemented);
    }

    #[test]
    fn test_quant_warnings() {
        let partial = QuantSupport::Partial;
        assert!(partial.warning_message("BnB").is_some());

        let stable = QuantSupport::Stable;
        assert!(stable.warning_message("GGUF").is_none());
    }

    #[test]
    fn test_compilation_hints() {
        let features = FeatureSet {
            flash_attn: false,
            flash_attn_v3: false,
            cuda: false,
            metal: false,
            accelerate: false,
            mkl: false,
            pyo3: false,
        };

        let hints = features.compilation_hints(&["flash-attn", "cuda"]);
        assert!(hints.contains("flash-attn"));
        assert!(hints.contains("cuda"));
    }
}
