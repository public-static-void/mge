use super::TimeOfDay;
use super::World;
use std::cell::RefCell;
use std::rc::Rc;

impl World {
    /// Register a system
    pub fn register_system<S: crate::ecs::system::System + 'static>(&mut self, system: S) {
        self.systems.register_system(system);
    }

    /// List all registered systems
    pub fn list_systems(&self) -> Vec<String> {
        let mut all = self.systems.list_systems();
        all.extend(self.dynamic_systems.list_systems());
        all
    }

    /// List all registered systems in dependency order
    pub fn list_systems_in_dependency_order(&self) -> Vec<String> {
        let mut all = self.systems.sorted_system_names();
        all.extend(self.dynamic_systems.list_systems());
        all
    }

    /// Check if a system is registered
    pub fn has_system(&self, name: &str) -> bool {
        self.systems.is_registered(name) || self.dynamic_systems.is_registered(name)
    }

    /// Register a dynamic system
    pub fn register_dynamic_system<F>(&mut self, name: &str, run: F)
    where
        F: Fn(Rc<RefCell<World>>, f32) + 'static,
    {
        self.dynamic_systems
            .register_system(name.to_string(), Box::new(run));
    }

    /// Register a dynamic system with dependencies
    pub fn register_dynamic_system_with_deps<F>(
        &mut self,
        name: &str,
        dependencies: Vec<String>,
        run: F,
    ) where
        F: Fn(Rc<RefCell<World>>, f32) + 'static,
    {
        self.dynamic_systems.register_system_with_deps(
            name.to_string(),
            dependencies,
            Box::new(run),
        );
    }

    /// Run a dynamic system
    pub fn run_dynamic_system(
        &self,
        world_rc: Rc<RefCell<World>>,
        name: &str,
    ) -> Result<(), String> {
        self.dynamic_systems.run_system(world_rc, name, 0.0)
    }

    /// Run a system
    pub fn run_system(&mut self, name: &str, lua: Option<&mlua::Lua>) -> Result<(), String> {
        if let Some(system) = self.systems.take_system(name) {
            system.borrow_mut().run(self, lua);
            self.systems.register_system_boxed(name.to_string(), system);
            Ok(())
        } else {
            Err(format!("System '{name}' not found"))
        }
    }

    /// Borrow-safe, idiomatic ECS tick.
    pub fn simulation_tick(world_rc: Rc<RefCell<World>>) {
        // Get the system names up front
        let system_names: Vec<String> = world_rc.borrow().systems.sorted_system_names();

        for name in &system_names {
            // Take the system out of the registry, drop the borrow immediately
            let cell = {
                let mut world = world_rc.borrow_mut();
                world.systems.take_system(name)
            };
            if let Some(cell) = cell {
                // Run the system, borrow the world only for this scope
                {
                    let mut world = world_rc.borrow_mut();
                    let mut system = cell.borrow_mut();
                    system.run(&mut world, None);
                }
                // Put the system back into the registry
                let mut world = world_rc.borrow_mut();
                world.systems.register_system_boxed(name.clone(), cell);
            }
        }

        // Dynamic systems
        let dynamic_names = world_rc.borrow().dynamic_systems.list_systems();
        for name in dynamic_names {
            let _ = world_rc
                .borrow()
                .run_dynamic_system(Rc::clone(&world_rc), &name);
        }

        {
            let mut world = world_rc.borrow_mut();
            world.update_event_buses::<serde_json::Value>();
            world.turn += 1;
        }
    }

    /// Borrow-safe, idiomatic ECS tick
    pub fn tick(world_rc: Rc<RefCell<World>>) {
        World::simulation_tick(Rc::clone(&world_rc));
        world_rc.borrow_mut().advance_time_of_day();
    }

    fn advance_time_of_day(&mut self) {
        self.time_of_day.minute += 1;
        if self.time_of_day.minute >= 60 {
            self.time_of_day.minute = 0;
            self.time_of_day.hour += 1;
            if self.time_of_day.hour >= 24 {
                self.time_of_day.hour = 0;
            }
        }
    }

    /// Get the current time of day
    pub fn get_time_of_day(&self) -> TimeOfDay {
        self.time_of_day
    }
}
