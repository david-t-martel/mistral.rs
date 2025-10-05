#[test]
fn test_git_bash_mangled_paths() {
    use winpath::{normalize_path, detect_path_format};

    let test_cases = vec![
        (
            r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe",
            r"C:\users\david\.local\bin\ls.exe",
        ),
        (
            r"C:\Program Files (x86)\Git\mnt\d\projects\test.txt",
            r"D:\projects\test.txt",
        ),
        (
            r"C:\Git\mnt\c\windows\system32",
            r"C:\windows\system32",
        ),
    ];

    for (input, expected) in test_cases {
        let format = detect_path_format(input);
        println!("Testing: {}", input);
        println!("Detected format: {:?}", format);

        let result = normalize_path(input);
        assert!(result.is_ok(), "Failed to normalize: {:?}", result);

        let normalized = result.unwrap();
        assert_eq!(
            normalized, expected,
            "\nInput: {}\nExpected: {}\nGot: {}",
            input, expected, normalized
        );
    }
}
