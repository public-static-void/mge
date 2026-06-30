use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::cell_key::CellKey;
use crate::map::fov::compute_fov;
use std::collections::HashSet;

/// System: Computes field-of-view for all entities with a Sight component.
/// Each tick, this system iterates entities with Sight, reads their Position,
/// and stores visible cell sets in `world.visible_cells`.
pub struct FovUpdateSystem;

impl System for FovUpdateSystem {
    fn name(&self) -> &'static str {
        "FovUpdateSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn run(&mut self, world: &mut World) {
        // Guard against missing map
        let map = match &world.map {
            Some(m) => m,
            None => return,
        };

        // Collect-then-apply pattern to avoid borrow conflicts with world.components
        let mut results: Vec<(u32, HashSet<CellKey>)> = Vec::new();

        if let Some(sight_components) = world.components.get("Sight") {
            for (&entity, data) in sight_components.iter() {
                let range = data
                    .get("range")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8) as u32;

                if let Some(pos) = world
                    .get_component(entity, "Position")
                    .and_then(|comp| CellKey::from_position(comp))
                {
                    let visible = compute_fov(map, &pos, range);
                    results.push((entity, visible));
                }
            }
        }

        // Apply collected results
        for (entity, visible) in results {
            world.set_visible_cells(entity, visible);
        }
    }
}
