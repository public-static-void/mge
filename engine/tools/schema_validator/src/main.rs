use schema_validator::validate_schema;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: schema_validator <schema-file-or-directory>");
        std::process::exit(1);
    }
    let path = Path::new(&args[1]);
    let mut failed = 0;
    let mut checked = 0;

    if path.is_file() {
        checked += 1;
        if let Err(e) = check_file(path) {
            failed += 1;
            eprintln!("\x1b[31m{}: ERROR: {}\x1b[0m", path.display(), e);
        } else {
            println!("\x1b[32m{}: OK\x1b[0m", path.display());
        }
    } else if path.is_dir() {
        for entry in fs::read_dir(path).expect("Failed to read directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                checked += 1;
                if let Err(e) = check_file(&path) {
                    failed += 1;
                    eprintln!("\x1b[31m{}: ERROR: {}\x1b[0m", path.display(), e);
                } else {
                    println!("\x1b[32m{}: OK\x1b[0m", path.display());
                }
            }
        }
    } else {
        eprintln!("Path does not exist: {}", path.display());
        std::process::exit(1);
    }

    println!("Checked {} files, {} errors.", checked, failed);
    if failed > 0 {
        std::process::exit(2);
    }
}

fn check_file(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    validate_schema(&content)
}
