//! File operation utilities from winutils
//!
//! Wrappers for: cp, mv, rm, mkdir, rmdir, touch

use super::wrapper::WinutilCommand;
use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::path::Path;

/// Copy files and directories
pub fn cp(sandbox: &Sandbox, source: &Path, dest: &Path, recursive: bool) -> AgentResult<()> {
    sandbox.validate_read(source)?;
    sandbox.validate_write(dest)?;

    let mut cmd = WinutilCommand::new("cp");

    if recursive {
        cmd = cmd.arg("-r");
    }

    cmd = cmd
        .arg(source.display().to_string())
        .arg(dest.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "cp failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(())
}

/// Move/rename files and directories
pub fn mv(sandbox: &Sandbox, source: &Path, dest: &Path) -> AgentResult<()> {
    sandbox.validate_read(source)?;
    sandbox.validate_write(dest)?;

    let cmd = WinutilCommand::new("mv")
        .arg(source.display().to_string())
        .arg(dest.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "mv failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(())
}

/// Remove files and directories
pub fn rm(sandbox: &Sandbox, path: &Path, recursive: bool, force: bool) -> AgentResult<()> {
    sandbox.validate_write(path)?;

    let mut cmd = WinutilCommand::new("rm");

    if recursive {
        cmd = cmd.arg("-r");
    }

    if force {
        cmd = cmd.arg("-f");
    }

    cmd = cmd.arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "rm failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(())
}

/// Create directory
pub fn mkdir(sandbox: &Sandbox, path: &Path, parents: bool) -> AgentResult<()> {
    sandbox.validate_write(path)?;

    let mut cmd = WinutilCommand::new("mkdir");

    if parents {
        cmd = cmd.arg("-p");
    }

    cmd = cmd.arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "mkdir failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(())
}

/// Remove empty directory
pub fn rmdir(sandbox: &Sandbox, path: &Path) -> AgentResult<()> {
    sandbox.validate_write(path)?;

    let cmd = WinutilCommand::new("rmdir").arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "rmdir failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(())
}

/// Update file timestamps or create empty file
pub fn touch(sandbox: &Sandbox, path: &Path) -> AgentResult<()> {
    sandbox.validate_write(path)?;

    let cmd = WinutilCommand::new("touch").arg(path.display().to_string());

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "touch failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(())
}
