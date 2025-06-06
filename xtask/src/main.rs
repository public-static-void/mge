use std::fs;
use std::path::Path;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xtask <command>");
        exit(1);
    }
    let command = &args[1];

    match command.as_str() {
        "build-plugins" => {
            if let Err(e) = build_and_deploy_plugins() {
                eprintln!("Error: {e}");
                exit(1);
            }
        }
        "build-c-plugins" => {
            if let Err(e) = build_c_plugins() {
                eprintln!("Error: {e}");
                exit(1);
            }
        }
        "build-wasm-tests" => {
            if let Err(e) = build_wasm_tests() {
                eprintln!("Error: {e}");
                exit(1);
            }
        }
        "build-all" => {
            if let Err(e) = build_and_deploy_plugins() {
                eprintln!("Error: {e}");
                exit(1);
            }
            if let Err(e) = build_c_plugins() {
                eprintln!("Error: {e}");
                exit(1);
            }
            if let Err(e) = build_wasm_tests() {
                eprintln!("Error: {e}");
                exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {command}");
            exit(1);
        }
    }
}

fn build_and_deploy_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let plugins_dir = Path::new("plugins");
    let target_os = std::env::consts::OS;
    let dylib_ext = match target_os {
        "linux" => "so",
        "macos" => "dylib",
        "windows" => "dll",
        _ => panic!("Unsupported OS"),
    };
    let lib_prefix = if target_os == "windows" { "" } else { "lib" };

    let workspace_target_dir = Path::new("target/release");

    for entry in fs::read_dir(plugins_dir)? {
        let entry = entry?;
        let crate_path = entry.path();
        if !crate_path.is_dir() {
            continue;
        }
        let cargo_toml = crate_path.join("Cargo.toml");
        if !cargo_toml.exists() {
            continue;
        }
        let crate_name = crate_path.file_name().unwrap().to_str().unwrap();
        let crate_package_name = get_package_name(&cargo_toml)?;
        println!("Building plugin crate: {crate_name}");

        // Build plugin (all targets, including binaries)
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&crate_path)
            .status()?;
        if !status.success() {
            return Err(format!("Build failed for {crate_name}").into());
        }

        // === Deploy dynamic library ===
        let dylib_name = format!("{lib_prefix}{crate_package_name}.{dylib_ext}");
        let dest_dylib = crate_path.join(&dylib_name);

        // 1. Try workspace target/release
        let built_dylib = workspace_target_dir.join(&dylib_name);
        if built_dylib.exists() {
            fs::copy(&built_dylib, &dest_dylib)?;
            println!(
                "Deployed {} to {}",
                built_dylib.display(),
                dest_dylib.display()
            );
        } else {
            // 2. Try workspace target/release/deps with hash
            let deps_dir = workspace_target_dir.join("deps");
            let found = fs::read_dir(&deps_dir)?.filter_map(|e| e.ok()).find(|e| {
                let fname = e.file_name().to_string_lossy().to_string();
                fname.starts_with(&format!("{}{}", lib_prefix, crate_package_name))
                    && fname.ends_with(dylib_ext)
            });
            if let Some(entry) = found {
                let built_dylib = entry.path();
                fs::copy(&built_dylib, &dest_dylib)?;
                println!(
                    "Deployed {} to {}",
                    built_dylib.display(),
                    dest_dylib.display()
                );
            } else {
                return Err(format!(
                    "Built library not found in {} or {}",
                    workspace_target_dir.display(),
                    deps_dir.display()
                )
                .into());
            }
        }

        // === Deploy binary executable (if it exists) ===
        let bin_name = if target_os == "windows" {
            format!("{}.exe", crate_package_name)
        } else {
            crate_package_name.clone()
        };
        let built_bin = workspace_target_dir.join(&bin_name);
        let dest_bin = crate_path.join(&bin_name);
        if built_bin.exists() {
            fs::copy(&built_bin, &dest_bin)?;
            println!("Deployed {} to {}", built_bin.display(), dest_bin.display());
        } else {
            // 2. Try workspace target/release/deps (rare, but possible for some setups)
            let deps_dir = workspace_target_dir.join("deps");
            let found = fs::read_dir(&deps_dir)?.filter_map(|e| e.ok()).find(|e| {
                let fname = e.file_name().to_string_lossy().to_string();
                fname.starts_with(&bin_name)
                    && (fname == bin_name || fname.starts_with(&format!("{}-", bin_name)))
            });
            if let Some(entry) = found {
                let built_bin = entry.path();
                fs::copy(&built_bin, &dest_bin)?;
                println!("Deployed {} to {}", built_bin.display(), dest_bin.display());
            } else {
                println!(
                    "Warning: Built binary {} not found in {} or {} (skipping binary deploy)",
                    bin_name,
                    workspace_target_dir.display(),
                    deps_dir.display()
                );
            }
        }
    }
    Ok(())
}

fn build_c_plugins() -> Result<(), Box<dyn std::error::Error>> {
    let plugins_dir = Path::new("plugins");
    let engine_dir = Path::new("engine");
    for entry in fs::read_dir(plugins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("c") {
            let base = path.file_stem().unwrap().to_str().unwrap();
            let out_lib = plugins_dir.join(format!("lib{}.so", base));
            println!("Compiling {} -> {}", path.display(), out_lib.display());
            let status = Command::new("gcc")
                .args([
                    "-I",
                    engine_dir.to_str().unwrap(),
                    "-shared",
                    "-fPIC",
                    path.to_str().unwrap(),
                    "-o",
                    out_lib.to_str().unwrap(),
                ])
                .status()?;
            if !status.success() {
                return Err(format!("Failed to compile {}", path.display()).into());
            }
        }
    }
    Ok(())
}

fn build_wasm_tests() -> Result<(), Box<dyn std::error::Error>> {
    let test_dir = Path::new("engine_wasm/wasm_tests");
    for entry in fs::read_dir(test_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let stem = path.file_stem().unwrap().to_str().unwrap();
            // Only compile files starting with "test_"
            if !stem.starts_with("test_") {
                continue;
            }
            let wasm_out = test_dir.join(format!("{stem}.wasm"));
            println!("Compiling {} to {}", path.display(), wasm_out.display());
            let status = Command::new("rustc")
                .args([
                    "--target",
                    "wasm32-unknown-unknown",
                    "-O",
                    "--crate-type=cdylib",
                    path.to_str().unwrap(),
                    "-o",
                    wasm_out.to_str().unwrap(),
                ])
                .status()?;
            if !status.success() {
                return Err(format!("Failed to compile {} to WASM", path.display()).into());
            }
        }
    }
    Ok(())
}

fn get_package_name(cargo_toml: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(cargo_toml)?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("name = ") {
            let name = rest.trim().trim_matches('"').to_string();
            return Ok(name);
        }
    }
    Err("No package name found in Cargo.toml".into())
}
