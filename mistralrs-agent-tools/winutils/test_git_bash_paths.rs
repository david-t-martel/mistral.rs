use winpath::{normalize_path, detect_path_format, PathFormat};

fn main() {
    let test_paths = vec![
        r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe",
        r"C:\Program Files (x86)\Git\mnt\d\projects\test.txt",
        r"C:\Git\mnt\c\windows\system32",
        r"C:\Program Files\Git/mnt/c/users/david/.local/bin/cat.exe",
        "/mnt/c/users/david/.local/bin/ls.exe",  // Regular WSL path for comparison
    ];

    println!("Testing Git Bash Path Normalization:\n");
    println!("{}", "=".repeat(80));

    for path in &test_paths {
        let format = detect_path_format(path);
        println!("\nInput:  {}", path);
        println!("Format: {:?}", format);

        match normalize_path(path) {
            Ok(normalized) => {
                println!("Output: {}", normalized);
                if format == PathFormat::GitBashMangled {
                    println!("✅ Git Bash mangled path correctly detected and normalized!");
                }
            },
            Err(e) => {
                println!("❌ Error: {:?}", e);
            }
        }
    }

    println!("\n{}", "=".repeat(80));
    println!("\nTest Summary:");
    println!("- Git Bash mangled paths should be detected as PathFormat::GitBashMangled");
    println!("- They should normalize to proper Windows paths without Git prefix");
    println!("- Example: C:\\Program Files\\Git\\mnt\\c\\users\\... -> C:\\users\\...");
}
