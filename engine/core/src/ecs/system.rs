pub trait System: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&mut self, world: &mut crate::scripting::world::World);
}

pub struct SystemRegistry {
    systems: std::collections::HashMap<String, Box<dyn System>>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: Default::default(),
        }
    }
    pub fn register_system<S: System + 'static>(&mut self, system: S) {
        self.systems
            .insert(system.name().to_string(), Box::new(system));
    }
    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }
    pub fn run_system(
        &mut self,
        name: &str,
        world: &mut crate::scripting::world::World,
    ) -> Result<(), String> {
        self.systems
            .get_mut(name)
            .map(|s| s.run(world))
            .ok_or_else(|| format!("System '{}' not found", name))
    }
}
