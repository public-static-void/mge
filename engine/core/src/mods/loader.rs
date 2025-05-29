use crate::ecs::schema::load_schemas_from_dir;
use crate::ecs::world::World;
use crate::scripting::ScriptEngine;
use std::cell::RefCell;
use std::rc::Rc;

pub fn load_mod(
    mod_dir: &str,
    world: Rc<RefCell<World>>,
    engine: &mut ScriptEngine,
) -> anyhow::Result<()> {
    // Load the mod manifest (mod.json)
    let manifest_path = format!("{}/mod.json", mod_dir);
    let manifest_data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| anyhow::anyhow!("Failed to read mod manifest: {}", e))?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse mod manifest: {}", e))?;

    // Load schemas using the unified loader
    let schema_dir = format!("{}/schemas", mod_dir);
    let schemas = load_schemas_from_dir(&schema_dir)
        .map_err(|e| anyhow::anyhow!("Failed to load schemas: {}", e))?;

    // Register schemas with the world's registry
    let registry = world.borrow().registry.clone();
    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    // Optionally: Load assets, jobs, recipes, etc. as needed here

    // Load and run the main system script (assume manifest has "main_script" field)
    if let Some(main_script) = manifest.get("main_script").and_then(|v| v.as_str()) {
        let script_path = format!("{}/{}", mod_dir, main_script);
        let script = std::fs::read_to_string(&script_path)
            .map_err(|e| anyhow::anyhow!("Failed to read main script: {}", e))?;
        if let Err(e) = engine.run_script(&script) {
            return Err(anyhow::anyhow!("Error running main script: {:?}", e));
        }
    } else {
        return Err(anyhow::anyhow!(
            "No main_script field found in mod manifest"
        ));
    }

    Ok(())
}
