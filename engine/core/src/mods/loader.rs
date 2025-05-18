use crate::ecs::world::World;
use crate::mods::manifest::ModManifest;
use crate::scripting::engine::ScriptEngine;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

pub fn load_mod<P: AsRef<Path>>(
    mod_dir: P,
    world: Rc<RefCell<World>>,
    script_engine: &mut ScriptEngine,
) -> Result<(), String> {
    let mod_dir = mod_dir.as_ref();
    let manifest_path = mod_dir.join("mod.json");
    let manifest_str =
        fs::read_to_string(&manifest_path).map_err(|e| format!("Failed to read manifest: {e}"))?;
    let manifest: ModManifest = serde_json::from_str(&manifest_str)
        .map_err(|e| format!("Failed to parse manifest: {e}"))?;

    // Register schemas
    for schema_rel in &manifest.schemas {
        let schema_path = mod_dir.join(schema_rel);
        let schema_str = fs::read_to_string(&schema_path)
            .map_err(|e| format!("Failed to read schema {schema_rel}: {e}"))?;
        let schema: crate::ecs::schema::ComponentSchema = serde_json::from_str(&schema_str)
            .map_err(|e| format!("Failed to parse schema {schema_rel}: {e}"))?;
        world
            .borrow()
            .registry
            .lock()
            .unwrap()
            .register_external_schema(schema);
    }

    // Register systems (Lua/Python only for now; extend for plugins as needed)
    for sys in &manifest.systems {
        let system_path = mod_dir.join(&sys.file);
        let ext = system_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        if ext == "lua" {
            let script_code = std::fs::read_to_string(&system_path)
                .map_err(|e| format!("Failed to read Lua system {}: {e}", sys.name))?;
            script_engine
                .run_script(&script_code)
                .map_err(|e| format!("Failed to load Lua system {}: {e}", sys.name))?;
        } else if ext == "py" {
            // Similar for Python
        } else {
            // For now, skip unknown system types
        }
    }

    // (Optionally) Load scripts similarly

    Ok(())
}
