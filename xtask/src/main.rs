use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xtask <command>");
        exit(1);
    }
    let command = &args[1];

    let result = match command.as_str() {
        "build-plugins" => build_and_deploy_plugins(),
        "build-c-plugins" => build_c_plugins(),
        "build-wasm-tests" => build_wasm_tests(),
        "build-all" => build_and_deploy_plugins()
            .and_then(|_| build_c_plugins())
            .and_then(|_| build_wasm_tests()),
        _ => Err(Box::from(format!("Unknown command: {command}"))),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        exit(1);
    }
}

fn build_and_deploy_plugins() -> Result<(), Box<dyn Error>> {
    let plugins_dir = Path::new("plugins");
    let target_os = std::env::consts::OS;
    let (dylib_ext, lib_prefix) = match target_os {
        "linux" => ("so", "lib"),
        "macos" => ("dylib", "lib"),
        "windows" => ("dll", ""),
        _ => panic!("Unsupported OS: {}", target_os),
    };

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
        let crate_package_name = get_package_name(&cargo_toml)?;
        println!("Building plugin crate: {}", crate_path.display());

        // Build plugin (all targets, including binaries)
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&crate_path)
            .status()?;
        if !status.success() {
            return Err(format!("Build failed for {}", crate_path.display()).into());
        }

        // Deploy dynamic library
        let dylib_name = format!("{}{}.{}", lib_prefix, crate_package_name, dylib_ext);
        let dest_dylib = crate_path.join(&dylib_name);
        let built_dylib = workspace_target_dir.join(&dylib_name);
        if built_dylib.exists() {
            fs::copy(&built_dylib, &dest_dylib)?;
            println!(
                "Deployed {} to {}",
                built_dylib.display(),
                dest_dylib.display()
            );
        } else {
            // Try deps dir with hash
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
                // Early exit on missing library
                return Err(format!(
                    "Built library not found in {} or {}",
                    workspace_target_dir.display(),
                    deps_dir.display()
                )
                .into());
            }
        }

        // Deploy binary executable (if it exists)
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
            // Try deps dir for executables (rare)
            let deps_dir = workspace_target_dir.join("deps");
            let found = fs::read_dir(&deps_dir)?.filter_map(|e| e.ok()).find(|e| {
                let fname = e.file_name().to_string_lossy().to_string();
                fname == bin_name || fname.starts_with(&format!("{}-", bin_name))
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

fn build_c_plugins() -> Result<(), Box<dyn Error>> {
    let plugins_dir = Path::new("plugins");
    let engine_dir = Path::new("engine");

    // Collect all possible include paths for robustness
    let mut include_paths = vec![
        engine_dir.to_path_buf(),
        PathBuf::from("/usr/include"),
        PathBuf::from("/usr/include/x86_64-linux-gnu"),
    ];
    // Add any custom include paths from env, if present
    if let Ok(path) = std::env::var("EXTRA_INCLUDE") {
        include_paths.push(PathBuf::from(path));
    }

    for entry in fs::read_dir(plugins_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Look for a single .c file in the plugin subdirectory
        let mut c_files = Vec::new();
        for file in fs::read_dir(&path)? {
            let file = file?;
            if file.path().extension().and_then(|e| e.to_str()) == Some("c") {
                c_files.push(file.path());
            }
        }
        if c_files.len() == 1 {
            let c_file = &c_files[0];
            let base = c_file.file_stem().unwrap().to_str().unwrap();
            let out_lib = path.join(format!("lib{}.so", base));
            println!("Compiling {} -> {}", c_file.display(), out_lib.display());

            // Build gcc command with all include paths
            let mut cmd = Command::new("gcc");
            for inc in &include_paths {
                cmd.arg("-I").arg(inc);
            }
            cmd.args([
                "-L",
                "/usr/lib/x86_64-linux-gnu",
                "-shared",
                "-fPIC",
                c_file.to_str().unwrap(),
                "-o",
                out_lib.to_str().unwrap(),
                "-ljansson",
            ]);

            let status = cmd.status()?;
            if !status.success() {
                return Err(format!("Failed to compile {}", c_file.display()).into());
            }
        } else if !c_files.is_empty() {
            return Err(format!(
                "Expected exactly one .c file in {}, found {}",
                path.display(),
                c_files.len()
            )
            .into());
        }
    }
    Ok(())
}

fn build_wasm_tests() -> Result<(), Box<dyn Error>> {
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

fn get_package_name(cargo_toml: &Path) -> Result<String, Box<dyn Error>> {
    let content = fs::read_to_string(cargo_toml)?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("name = ") {
            let name = rest.trim().trim_matches('"').to_string();
            return Ok(name);
        }
    }
    Err("No package name found in Cargo.toml".into())
}
