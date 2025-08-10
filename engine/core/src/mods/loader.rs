use crate::ecs::schema::load_allowed_modes;
use crate::ecs::schema::load_schemas_from_dir_with_modes;
use crate::ecs::world::World;
use std::cell::RefCell;
use std::rc::Rc;

/// Loads a mod, registers schemas, and runs the main script via a scripting engine passed in.
pub fn load_mod<S: ModScriptEngine>(
    mod_dir: &str,
    world: Rc<RefCell<World>>,
    engine: &mut S,
) -> anyhow::Result<()> {
    // Load the mod manifest (mod.json)
    let manifest_path = format!("{mod_dir}/mod.json");
    let manifest_data = std::fs::read_to_string(&manifest_path)
        .map_err(|e| anyhow::anyhow!("Failed to read mod manifest: {}", e))?;
    let manifest: serde_json::Value = serde_json::from_str(&manifest_data)
        .map_err(|e| anyhow::anyhow!("Failed to parse mod manifest: {}", e))?;

    // Load schemas using the unified loader
    let schema_dir = format!("{mod_dir}/schemas");
    let allowed_modes = load_allowed_modes()?;
    let schemas = load_schemas_from_dir_with_modes(&schema_dir, &allowed_modes)
        .map_err(|e| anyhow::anyhow!("Failed to load schemas: {}", e))?;

    // Register schemas with the world's registry
    let registry = world.borrow().registry.clone();
    for (_name, schema) in schemas {
        registry.lock().unwrap().register_external_schema(schema);
    }

    // Optionally: Load assets, jobs, recipes, etc. as needed here

    // Load and run the main system script (assume manifest has "main_script" field)
    if let Some(main_script) = manifest.get("main_script").and_then(|v| v.as_str()) {
        let script_path = format!("{mod_dir}/{main_script}");
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

/// Trait for scripting engines that can run mod scripts.
pub trait ModScriptEngine {
    /// Runs a mod script.
    fn run_script(&mut self, script: &str) -> Result<(), String>;
}
