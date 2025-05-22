use super::World;

impl World {
    pub fn register_system<S: crate::ecs::system::System + 'static>(&mut self, system: S) {
        self.systems.register_system(system);
    }

    pub fn list_systems(&self) -> Vec<String> {
        let mut all = self.systems.list_systems();
        all.extend(self.dynamic_systems.list_systems());
        all
    }

    pub fn list_systems_in_dependency_order(&self) -> Vec<String> {
        let mut all = self.systems.sorted_system_names();
        all.extend(self.dynamic_systems.list_systems());
        all
    }

    pub fn register_dynamic_system<F>(&mut self, name: &str, run: F)
    where
        F: Fn(&mut World, f32) + 'static,
    {
        self.dynamic_systems
            .register_system(name.to_string(), Box::new(run));
    }

    pub fn register_dynamic_system_with_deps<F>(
        &mut self,
        name: &str,
        dependencies: Vec<String>,
        run: F,
    ) where
        F: Fn(&mut World, f32) + 'static,
    {
        self.dynamic_systems.register_system_with_deps(
            name.to_string(),
            dependencies,
            Box::new(run),
        );
    }

    pub fn run_dynamic_system(&mut self, name: &str) -> Result<(), String> {
        let name = name.to_string();
        let dynamic_systems = std::mem::take(&mut self.dynamic_systems);
        let result = dynamic_systems.run_system(self, &name, 0.0);
        self.dynamic_systems = dynamic_systems;
        result
    }

    pub fn run_system(&mut self, name: &str, lua: Option<&mlua::Lua>) -> Result<(), String> {
        if let Some(system) = self.systems.take_system(name) {
            system.borrow_mut().run(self, lua);
            self.systems.register_system_boxed(name.to_string(), system);
            Ok(())
        } else {
            let dynamic_systems = std::mem::take(&mut self.dynamic_systems);
            let result = dynamic_systems.run_system(self, name, 0.0);
            self.dynamic_systems = dynamic_systems;
            result
        }
    }

    pub fn simulation_tick(&mut self) {
        let system_names: Vec<String> = self.systems.sorted_system_names();
        for name in &system_names {
            if let Some(cell) = self.systems.take_system(name) {
                {
                    let mut system = cell.borrow_mut();
                    system.run(self, None);
                }
                self.systems.register_system_boxed(name.clone(), cell);
            }
        }

        let dynamic_names = self.dynamic_systems.list_systems();
        for name in dynamic_names {
            let _ = self.run_dynamic_system(&name);
        }

        self.update_event_buses::<serde_json::Value>();
        self.turn += 1;
    }
}
