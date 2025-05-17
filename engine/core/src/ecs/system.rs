use crate::scripting::world::World;
use indexmap::IndexMap;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};

pub trait System: Send + Sync {
    fn name(&self) -> &'static str;
    fn run(&mut self, world: &mut World);
    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }
}

pub struct SystemRegistry {
    systems: IndexMap<String, RefCell<Box<dyn System>>>,
}

impl SystemRegistry {
    pub fn new() -> Self {
        Self {
            systems: IndexMap::new(),
        }
    }

    pub fn register_system<S: System + 'static>(&mut self, system: S) {
        self.systems.insert(
            system.name().to_string(),
            std::cell::RefCell::new(Box::new(system)),
        );
    }

    pub fn take_system(&mut self, name: &str) -> Option<std::cell::RefCell<Box<dyn System>>> {
        self.systems.shift_remove(name)
    }

    pub fn register_system_boxed(
        &mut self,
        name: String,
        system: std::cell::RefCell<Box<dyn System>>,
    ) {
        self.systems.insert(name, system);
    }

    pub fn list_systems(&self) -> Vec<String> {
        self.systems.keys().cloned().collect()
    }

    pub fn get_system_mut(&self, name: &str) -> Option<std::cell::RefMut<Box<dyn System>>> {
        self.systems.get(name).map(|cell| cell.borrow_mut())
    }

    /// Returns a topologically sorted list of system names, or panics if a cycle is detected.
    pub fn sorted_system_names(&self) -> Vec<String> {
        // Build dependency graph: name -> set of dependencies
        let mut graph: HashMap<String, HashSet<String>> = HashMap::new();
        for (name, cell) in &self.systems {
            let deps = cell.borrow().dependencies();
            graph
                .entry(name.clone())
                .or_default()
                .extend(deps.iter().map(|&s| s.to_string()));
        }

        // Kahn's algorithm for topological sort
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for (name, deps) in &graph {
            in_degree.entry(name.clone()).or_insert(0);
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter_map(|(name, &deg)| if deg == 0 { Some(name.clone()) } else { None })
            .collect();
        let mut sorted = Vec::new();

        while let Some(name) = queue.pop_front() {
            sorted.push(name.clone());
            if let Some(deps) = graph.get(&name) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() != self.systems.len() {
            panic!("Cycle detected in system dependencies!");
        }

        // Reverse to get the correct order (from roots to leaves)
        sorted.reverse();

        // Only return systems that are registered (ignore unknown dependencies)
        sorted
            .into_iter()
            .filter(|name| self.systems.contains_key(name))
            .collect()
    }
}

impl Default for SystemRegistry {
    fn default() -> Self {
        Self::new()
    }
}
