//! Encoding utilities from winutils
//!
//! Wrappers for: base64, base32, basenc

use super::wrapper::WinutilCommand;
use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::path::Path;

/// Base64 encode/decode
pub fn base64(sandbox: &Sandbox, path: &Path, decode: bool) -> AgentResult<String> {
    sandbox.validate_read(path)?;

    let mut cmd = WinutilCommand::new("base64");

    if decode {
        cmd = cmd.arg("--decode");
    }

    cmd = cmd.arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "base64 failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Base32 encode/decode
pub fn base32(sandbox: &Sandbox, path: &Path, decode: bool) -> AgentResult<String> {
    sandbox.validate_read(path)?;

    let mut cmd = WinutilCommand::new("base32");

    if decode {
        cmd = cmd.arg("--decode");
    }

    cmd = cmd.arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "base32 failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Basenc encode/decode with multiple encodings
pub fn basenc(sandbox: &Sandbox, path: &Path, encoding: &str, decode: bool) -> AgentResult<String> {
    sandbox.validate_read(path)?;

    let mut cmd = WinutilCommand::new("basenc").arg(format!("--{}", encoding));

    if decode {
        cmd = cmd.arg("--decode");
    }

    cmd = cmd.arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "basenc failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}
