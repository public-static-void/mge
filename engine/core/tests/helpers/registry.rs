use std::fs;
use std::path::Path;

/// Loads a single schema from the assets/schemas directory by name.
/// Panics if the schema cannot be loaded.
pub fn load_schema_from_assets(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../engine/assets/schemas")
        .join(format!("{}.json", name));
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("Failed to read schema file: {:?}", path))
}
