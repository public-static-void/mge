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

    /// Path to game.toml config
    #[arg(long, value_name = "CONFIG", default_value = "game.toml")]
    config: PathBuf,

    /// Stop at first error
    #[arg(long)]
    fail_fast: bool,

    /// Only print summary at the end
    #[arg(long)]
    summary_only: bool,
}

fn main() {
    let args = Args::parse();

    // Load allowed modes from game.toml
    let allowed_modes = load_allowed_modes(&args.config);

    let mut failed = 0;
    let mut checked = 0;

    if args.path.is_file() {
        checked += 1;
        match check_file(&args.path, &allowed_modes) {
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
                match check_file(&path, &allowed_modes) {
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

fn check_file(path: &PathBuf, allowed_modes: &[String]) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {e}"))?;
    validate_schema(&content, allowed_modes)
}

fn print_summary(checked: usize, failed: usize) {
    println!("Checked {checked} files, {failed} errors.");
}

// --- Load allowed_modes from game.toml ---
fn load_allowed_modes(config_path: &PathBuf) -> Vec<String> {
    let content = fs::read_to_string(config_path)
        .unwrap_or_else(|_| panic!("Failed to read config file: {}", config_path.display()));
    let value: toml::Value = toml::from_str(&content)
        .unwrap_or_else(|_| panic!("Failed to parse TOML config: {}", config_path.display()));
    value
        .get("allowed_modes")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}
