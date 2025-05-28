use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, exit};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: xtask <cmd>");
        eprintln!("Commands: build-plugins [plugin_name]");
        exit(1);
    }

    match args[1].as_str() {
        "build-plugins" => {
            let plugin_name = args.get(2).map(String::as_str);
            if let Err(e) = build_and_deploy_plugins(plugin_name) {
                eprintln!("xtask error: {e}");
                exit(1);
            }
        }
        cmd => {
            eprintln!("Unknown command: {cmd}");
            exit(1);
        }
    }
}

fn build_and_deploy_plugins(plugin_name: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let plugins_dir = Path::new("plugins");
    let plugin_crates = fs::read_dir(plugins_dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_dir() && path.join("Cargo.toml").exists() {
                Some(path)
            } else {
                None
            }
        })
        .filter(|path| plugin_name.is_none_or(|name| path.ends_with(name)));

    let target_os = std::env::consts::OS;
    let dylib_ext = match target_os {
        "linux" => "so",
        "macos" => "dylib",
        "windows" => "dll",
        _ => panic!("Unsupported OS"),
    };

    // Workspace-wide target directory
    let workspace_target_dir = Path::new("target/release");

    for crate_path in plugin_crates {
        let crate_name = crate_path.file_name().unwrap().to_str().unwrap();
        let cargo_toml = crate_path.join("Cargo.toml");
        let crate_package_name = get_package_name(&cargo_toml)?;
        println!("Building plugin crate: {crate_name}");

        // Build plugin
        let status = Command::new("cargo")
            .arg("build")
            .arg("--release")
            .current_dir(&crate_path)
            .status()?;
        if !status.success() {
            return Err(format!("Build failed for {crate_name}").into());
        }

        let lib_prefix = if target_os == "windows" { "" } else { "lib" };
        let dylib_name = format!("{lib_prefix}{crate_package_name}.{dylib_ext}");
        let dest = crate_path.join(&dylib_name);

        // 1. Try workspace target/release
        let built_lib = workspace_target_dir.join(&dylib_name);
        if built_lib.exists() {
            fs::copy(&built_lib, &dest)?;
            println!(
                "Deployed {} to {}",
                built_lib.display(),
                crate_path.display()
            );
            continue;
        }

        // 2. Try workspace target/release/deps with hash
        let deps_dir = workspace_target_dir.join("deps");
        let found = std::fs::read_dir(&deps_dir)?
            .filter_map(|e| e.ok())
            .find(|e| {
                let binding = e.file_name();
                let fname = binding.to_string_lossy();
                fname.starts_with(&format!("{}{}", lib_prefix, crate_package_name))
                    && fname.ends_with(dylib_ext)
            });
        if let Some(entry) = found {
            let built_lib = entry.path();
            fs::copy(&built_lib, &dest)?;
            println!(
                "Deployed {} to {}",
                built_lib.display(),
                crate_path.display()
            );
            continue;
        }

        return Err(format!(
            "Built library not found in {} or {}",
            workspace_target_dir.display(),
            deps_dir.display()
        )
        .into());
    }
    Ok(())
}

fn get_package_name(cargo_toml: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(cargo_toml)?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name") && trimmed.contains('=') {
            // e.g. name = "rust_test_plugin"
            let name = trimmed.split('=').nth(1).unwrap().trim().trim_matches('"');
            return Ok(name.to_string());
        }
    }
    Err("No package name found in Cargo.toml".into())
}
