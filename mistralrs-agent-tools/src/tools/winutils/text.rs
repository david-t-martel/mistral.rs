//! Text processing utilities from winutils
//!
//! Wrappers for: cut, tr, expand, unexpand, fold, fmt, nl, tac

use super::wrapper::WinutilCommand;
use crate::tools::sandbox::Sandbox;
use crate::types::{AgentError, AgentResult};
use std::path::Path;

/// Extract selected fields from each line of files
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `paths` - Files to process
/// * `fields` - Field specification (e.g., "1,2,3" or "1-5")
/// * `delimiter` - Field delimiter (default: tab)
///
/// # Example
/// ```no_run
/// use mistralrs_agent_tools::tools::winutils::text::cut;
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::types::{AgentResult, SandboxConfig};
/// use std::path::Path;
///
/// fn main() -> AgentResult<()> {
///     let sandbox = Sandbox::new(SandboxConfig::default());
///     let output = cut(&sandbox, &[Path::new("data.csv")], "1,3,5", Some(','))?;
///     Ok(())
/// }
/// ```
pub fn cut(
    sandbox: &Sandbox,
    paths: &[&Path],
    fields: &str,
    delimiter: Option<char>,
) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("cut").arg("-f").arg(fields);

    if let Some(delim) = delimiter {
        cmd = cmd.arg("-d").arg(delim.to_string());
    }

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "cut failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Translate or delete characters
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `paths` - Files to process
/// * `from_chars` - Characters to translate from
/// * `to_chars` - Characters to translate to (or None to delete)
///
/// # Example
/// ```no_run
/// use mistralrs_agent_tools::tools::winutils::text::tr;
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::types::{AgentResult, SandboxConfig};
/// use std::path::Path;
///
/// fn main() -> AgentResult<()> {
///     let sandbox = Sandbox::new(SandboxConfig::default());
///     let output = tr(&sandbox, &[Path::new("file.txt")], "a-z", Some("A-Z"))?;
///     Ok(())
/// }
/// ```
pub fn tr(
    sandbox: &Sandbox,
    paths: &[&Path],
    from_chars: &str,
    to_chars: Option<&str>,
) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = if let Some(to) = to_chars {
        WinutilCommand::new("tr").arg(from_chars).arg(to)
    } else {
        WinutilCommand::new("tr").arg("-d").arg(from_chars)
    };

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "tr failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Convert tabs to spaces
///
/// # Arguments
/// * `sandbox` - Sandbox for path validation
/// * `paths` - Files to process
/// * `tab_stops` - Tab stop positions (default: 8)
///
/// # Example
/// ```no_run
/// use mistralrs_agent_tools::tools::winutils::text::expand;
/// use mistralrs_agent_tools::tools::sandbox::Sandbox;
/// use mistralrs_agent_tools::types::{AgentResult, SandboxConfig};
/// use std::path::Path;
///
/// fn main() -> AgentResult<()> {
///     let sandbox = Sandbox::new(SandboxConfig::default());
///     let output = expand(&sandbox, &[Path::new("file.txt")], Some(4))?;
///     Ok(())
/// }
/// ```
pub fn expand(sandbox: &Sandbox, paths: &[&Path], tab_stops: Option<usize>) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("expand");

    if let Some(stops) = tab_stops {
        cmd = cmd.arg("-t").arg(stops.to_string());
    }

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "expand failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Convert spaces to tabs
pub fn unexpand(
    sandbox: &Sandbox,
    paths: &[&Path],
    tab_stops: Option<usize>,
) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("unexpand");

    if let Some(stops) = tab_stops {
        cmd = cmd.arg("-t").arg(stops.to_string());
    }

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "unexpand failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Wrap lines to fit specified width
pub fn fold(sandbox: &Sandbox, paths: &[&Path], width: Option<usize>) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("fold");

    if let Some(w) = width {
        cmd = cmd.arg("-w").arg(w.to_string());
    }

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "fold failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Simple text formatter
pub fn fmt(sandbox: &Sandbox, paths: &[&Path], width: Option<usize>) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("fmt");

    if let Some(w) = width {
        cmd = cmd.arg("-w").arg(w.to_string());
    }

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "fmt failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Number lines of files
pub fn nl(sandbox: &Sandbox, paths: &[&Path], start: Option<usize>) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("nl");

    if let Some(n) = start {
        cmd = cmd.arg("-v").arg(n.to_string());
    }

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "nl failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

/// Reverse lines of files (concatenate in reverse)
pub fn tac(sandbox: &Sandbox, paths: &[&Path]) -> AgentResult<String> {
    if paths.is_empty() {
        return Err(AgentError::InvalidInput("No paths provided".to_string()));
    }

    let mut cmd = WinutilCommand::new("tac");

    for path in paths {
        sandbox.validate_read(path)?;
        cmd = cmd.arg(path.display().to_string());
    }

    let result = cmd.execute(sandbox)?;

    if result.status != 0 {
        return Err(AgentError::IoError(format!(
            "tac failed with status {}: {}",
            result.status, result.stderr
        )));
    }

    Ok(result.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SandboxConfig;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    #[ignore] // Requires winutils to be built
    fn test_tac() {
        let temp_dir = TempDir::new().unwrap();
        let file = create_test_file(temp_dir.path(), "test.txt", "line1\nline2\nline3\n");

        let config = SandboxConfig::new(temp_dir.path().to_path_buf());
        let sandbox = Sandbox::new(config);

        let result = tac(&sandbox, &[&file]).unwrap();

        assert!(result.contains("line3"));
        assert!(result.contains("line2"));
        assert!(result.contains("line1"));
    }
}
