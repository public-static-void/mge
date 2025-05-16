use crate::scripting::World;
use std::collections::HashMap;
use std::sync::Arc;

pub struct DynamicSystem {
    pub name: String,
    pub run: Box<dyn Fn(&mut World, f32) + Send + Sync>,
}

#[derive(Default)]
pub struct DynamicSystemRegistry {
    systems: HashMap<String, Arc<DynamicSystem>>,
}

impl DynamicSystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: HashMap::new(),
        }
    }

    pub fn register_system(
        &mut self,
        name: String,
        run: Box<dyn Fn(&mut World, f32) + Send + Sync>,
    ) {
        let system = Arc::new(DynamicSystem {
            name: name.clone(),
            run,
        });
        self.systems.insert(name, system);
    }

    pub fn run_system(&self, world: &mut World, name: &str, delta_time: f32) -> Result<(), String> {
        if let Some(system) = self.systems.get(name) {
            (system.run)(world, delta_time);
            Ok(())
        } else {
            Err(format!("Dynamic system '{}' not found", name))
        }
    }

    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }
}
