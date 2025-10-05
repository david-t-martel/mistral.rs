use winpath::normalize_path;

fn main() {
    let test_paths = vec![
        (r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe",
         r"C:\users\david\.local\bin\ls.exe"),
        (r"C:\Program Files (x86)\Git\mnt\d\projects\test.txt",
         r"D:\projects\test.txt"),
        (r"C:\Git\mnt\c\windows\system32",
         r"C:\windows\system32"),
        ("/mnt/c/users/david/.local/bin/ls.exe",
         r"C:\users\david\.local\bin\ls.exe"),
    ];

    println!("Testing Git Bash Path Normalization:\n");
    println!("{}", "=".repeat(80));

    for (input, expected) in &test_paths {
        println!("\nInput:    {}", input);

        match normalize_path(input) {
            Ok(normalized) => {
                println!("Output:   {}", normalized);
                println!("Expected: {}", expected);

                if &normalized == expected {
                    println!("✅ PASS - Path normalized correctly!");
                } else {
                    println!("❌ FAIL - Path normalization incorrect!");
                }
            },
            Err(e) => {
                println!("❌ Error: {:?}", e);
            }
        }
    }

    println!("\n{}", "=".repeat(80));
}
