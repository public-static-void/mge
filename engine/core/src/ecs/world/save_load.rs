use super::World;
use crate::ecs::registry::ComponentRegistry;
use serde_json::{Map, Value as JsonValue};
use std::sync::{Arc, Mutex};

impl World {
    /// Save the world to a file
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)
    }

    /// Load a world from a file, with migration of deprecated fields.
    pub fn load_from_file(
        path: &std::path::Path,
        registry: Arc<Mutex<ComponentRegistry>>,
    ) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        let mut world: Self = serde_json::from_str(&json)?;
        world.registry = registry;
        // Run migrations after loading
        world.migrate_agent_skills();
        Ok(world)
    }

    /// Migrate deprecated `agent.skills` to `SkillLevels` component (R006).
    ///
    /// If an entity has an `Agent` component with `skills` populated but no `SkillLevels`
    /// component, auto-populate `SkillLevels.skills` from `agent.skills`.
    /// If the entity already has `SkillLevels`, it takes precedence (no migration).
    /// Logs a deprecation warning when migration occurs.
    pub fn migrate_agent_skills(&mut self) {
        let agent_entities: Vec<u32> = self
            .components
            .get("Agent")
            .map(|map| map.keys().copied().collect())
            .unwrap_or_default();

        for &eid in &agent_entities {
            // Skip if SkillLevels already exists (EC-006)
            if self
                .components
                .get("SkillLevels")
                .and_then(|map| map.get(&eid))
                .is_some()
            {
                continue;
            }

            // Check for deprecated agent.skills
            if let Some(agent) = self.components.get("Agent").and_then(|map| map.get(&eid)) {
                if let Some(skills) = agent.get("skills").and_then(|v| v.as_object()) {
                    if !skills.is_empty() {
                        log::warn!(
                            "DEPRECATION: entity {eid} uses agent.skills — migrating to SkillLevels component. \
                             agent.skills will be removed in a future milestone."
                        );

                        let mut skill_levels_map = Map::new();
                        skill_levels_map
                            .insert("skills".to_string(), JsonValue::Object(skills.clone()));
                        skill_levels_map.insert("total_xp".to_string(), JsonValue::from(0.0));
                        skill_levels_map
                            .insert("skill_xp".to_string(), JsonValue::Object(Map::new()));
                        skill_levels_map
                            .insert("skill_levels".to_string(), JsonValue::Object(Map::new()));

                        self.components
                            .entry("SkillLevels".to_string())
                            .or_default()
                            .insert(eid, JsonValue::Object(skill_levels_map));
                    }
                }
            }
        }
    }
}
