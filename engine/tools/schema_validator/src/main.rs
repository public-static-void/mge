use clap::Parser;
use schema_validator::validate_schema;
use std::fs;
use std::path::PathBuf;

/// Schema Validator CLI
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// File or directory to validate
    #[arg(value_name = "PATH")]
    path: PathBuf,

    /// Stop at first error
    #[arg(long)]
    fail_fast: bool,

    /// Only print summary at the end
    #[arg(long)]
    summary_only: bool,
}

fn main() {
    let args = Args::parse();
    let mut failed = 0;
    let mut checked = 0;

    if args.path.is_file() {
        checked += 1;
        match check_file(&args.path) {
            Ok(_) => {
                if !args.summary_only {
                    println!("\x1b[32m{}: OK\x1b[0m", args.path.display());
                }
            }
            Err(e) => {
                failed += 1;
                if !args.summary_only {
                    eprintln!("\x1b[31m{}: ERROR: {}\x1b[0m", args.path.display(), e);
                }
                if args.fail_fast {
                    print_summary(checked, failed);
                    std::process::exit(2);
                }
            }
        }
    } else if args.path.is_dir() {
        for entry in fs::read_dir(&args.path).expect("Failed to read directory") {
            let entry = entry.expect("Failed to read entry");
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                checked += 1;
                match check_file(&path) {
                    Ok(_) => {
                        if !args.summary_only {
                            println!("\x1b[32m{}: OK\x1b[0m", path.display());
                        }
                    }
                    Err(e) => {
                        failed += 1;
                        if !args.summary_only {
                            eprintln!("\x1b[31m{}: ERROR: {}\x1b[0m", path.display(), e);
                        }
                        if args.fail_fast {
                            print_summary(checked, failed);
                            std::process::exit(2);
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Path does not exist: {}", args.path.display());
        std::process::exit(1);
    }

    print_summary(checked, failed);
    if failed > 0 {
        std::process::exit(2);
    }
}

fn check_file(path: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    validate_schema(&content)
}

fn print_summary(checked: usize, failed: usize) {
    println!("Checked {} files, {} errors.", checked, failed);
}
