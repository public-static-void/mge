use crate::scripting::World;
use indexmap::IndexMap;

pub type DynSystemFn = Box<dyn Fn(&mut World, f32) + 'static>;

#[derive(Default)]
pub struct DynamicSystemRegistry {
    systems: IndexMap<String, DynSystemFn>,
}

impl DynamicSystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: IndexMap::new(),
        }
    }

    pub fn register_system(&mut self, name: String, run: DynSystemFn) {
        self.systems.insert(name, run);
    }

    pub fn register_system_with_deps(
        &mut self,
        name: String,
        _dependencies: Vec<String>,
        run: DynSystemFn,
    ) {
        // TODO: Store dependencies if needed
        self.systems.insert(name, run);
    }

    pub fn run_system(&self, world: &mut World, name: &str, delta_time: f32) -> Result<(), String> {
        if let Some(system) = self.systems.get(name) {
            (system)(world, delta_time);
            Ok(())
        } else {
            Err(format!("Dynamic system '{}' not found", name))
        }
    }

    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }
}
