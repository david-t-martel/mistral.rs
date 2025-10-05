use winpath::{detect_path_format, normalize_path};

fn main() {
    let test_path = r"C:\Program Files\Git\mnt\c\users\david\.local\bin\ls.exe";

    println!("Testing path: {}", test_path);
    let format = detect_path_format(test_path);
    println!("Detected format: {:?}", format);

    match normalize_path(test_path) {
        Ok(normalized) => println!("Normalized to: {}", normalized),
        Err(e) => println!("Error: {:?}", e),
    }
}
