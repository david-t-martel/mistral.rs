//! Cat utility - concatenate and display files
//!
//! Adapted from winutils cat with agent tools integration.

use crate::tools::sandbox::Sandbox;
use crate::types::{AgentResult, Bom, CatOptions};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Read and concatenate files
pub fn cat(sandbox: &Sandbox, paths: &[&Path], options: &CatOptions) -> AgentResult<String> {
    let mut output = String::new();
    let mut line_number = 1;
    let mut last_line_blank = false;

    for path in paths {
        // Validate path through sandbox
        let validated_path = sandbox.validate_read(path)?;

        // Check file size
        sandbox.validate_file_size(&validated_path)?;

        // Open and read file
        let content = read_file_content(&validated_path, options)?;

        // Process content line by line
        for line in content.lines() {
            let is_blank = line.trim().is_empty();

            // Skip consecutive blank lines if squeeze_blank is enabled
            if options.squeeze_blank && is_blank && last_line_blank {
                continue;
            }

            // Add line number if requested
            if options.number_lines {
                output.push_str(&format!("{:6}\t", line_number));
                line_number += 1;
            }

            // Add the line content
            output.push_str(line);

            // Add end marker if requested
            if options.show_ends {
                output.push('$');
            }

            output.push('\n');
            last_line_blank = is_blank;
        }
    }

    Ok(output)
}

/// Read file content with encoding detection
fn read_file_content(path: &Path, _options: &CatOptions) -> AgentResult<String> {
    let mut file = File::open(path)?;

    // Read initial bytes for BOM detection
    let mut buffer = vec![0u8; 4096];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);

    // Detect BOM
    let bom = Bom::detect(&buffer);
    let start_offset = bom.size();

    // Reset file to after BOM
    drop(file);
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    // Skip BOM bytes
    if start_offset > 0 {
        let mut skip_buf = vec![0u8; start_offset];
        reader.read_exact(&mut skip_buf)?;
    }

    // Read rest of file
    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    Ok(content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentError, SandboxConfig};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_sandbox() -> (Sandbox, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let sandbox = Sandbox::new(SandboxConfig::new(temp_dir.path().to_path_buf()));
        (sandbox, temp_dir)
    }

    #[test]
    fn test_cat_single_file() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Hello, World!").unwrap();
        writeln!(file, "Second line").unwrap();

        let options = CatOptions::default();
        let result = cat(&sandbox, &[&file_path], &options).unwrap();

        assert_eq!(result, "Hello, World!\nSecond line\n");
    }

    #[test]
    fn test_cat_with_line_numbers() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();

        let options = CatOptions {
            number_lines: true,
            ..Default::default()
        };
        let result = cat(&sandbox, &[&file_path], &options).unwrap();

        assert!(result.contains("     1\tLine 1"));
        assert!(result.contains("     2\tLine 2"));
    }

    #[test]
    fn test_cat_with_show_ends() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file, "Line 2").unwrap();

        let options = CatOptions {
            show_ends: true,
            ..Default::default()
        };
        let result = cat(&sandbox, &[&file_path], &options).unwrap();

        assert!(result.contains("Line 1$\n"));
        assert!(result.contains("Line 2$\n"));
    }

    #[test]
    fn test_cat_squeeze_blank() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Line 1").unwrap();
        writeln!(file).unwrap(); // Blank
        writeln!(file).unwrap(); // Blank
        writeln!(file, "Line 2").unwrap();

        let options = CatOptions {
            squeeze_blank: true,
            ..Default::default()
        };
        let result = cat(&sandbox, &[&file_path], &options).unwrap();

        // Should have Line 1, one blank, then Line 2
        let line_count = result.lines().count();
        assert_eq!(line_count, 3); // Line 1, blank, Line 2
    }

    #[test]
    fn test_cat_multiple_files() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file1_path = temp_dir.path().join("test1.txt");
        let file2_path = temp_dir.path().join("test2.txt");

        let mut file1 = File::create(&file1_path).unwrap();
        writeln!(file1, "File 1").unwrap();

        let mut file2 = File::create(&file2_path).unwrap();
        writeln!(file2, "File 2").unwrap();

        let options = CatOptions::default();
        let result = cat(&sandbox, &[&file1_path, &file2_path], &options).unwrap();

        assert!(result.contains("File 1"));
        assert!(result.contains("File 2"));
    }

    #[test]
    fn test_cat_with_bom() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("test_bom.txt");
        let mut file = File::create(&file_path).unwrap();

        // Write UTF-8 BOM
        file.write_all(&[0xEF, 0xBB, 0xBF]).unwrap();
        writeln!(file, "Content after BOM").unwrap();

        let options = CatOptions::default();
        let result = cat(&sandbox, &[&file_path], &options).unwrap();

        // BOM should be stripped
        assert!(!result.starts_with("\u{FEFF}"));
        assert!(result.contains("Content after BOM"));
    }

    #[test]
    fn test_cat_sandbox_violation() {
        let (sandbox, _temp_dir) = create_test_sandbox();
        // Use a path outside the sandbox - on Windows use a different drive or system path
        #[cfg(windows)]
        let outside_path = PathBuf::from("C:\\Windows\\System32\\drivers\\etc\\hosts");
        #[cfg(not(windows))]
        let outside_path = PathBuf::from("/tmp/outside.txt");

        let options = CatOptions::default();
        let result = cat(&sandbox, &[&outside_path], &options);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AgentError::SandboxViolation(_)
        ));
    }

    #[test]
    fn test_cat_nonexistent_file() {
        let (sandbox, temp_dir) = create_test_sandbox();
        let file_path = temp_dir.path().join("nonexistent.txt");

        let options = CatOptions::default();
        let result = cat(&sandbox, &[&file_path], &options);

        assert!(result.is_err());
    }
}
