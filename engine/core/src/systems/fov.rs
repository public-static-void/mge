use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::cell_key::CellKey;
use crate::map::fov::{BfsFovAlgorithm, RecursiveShadowcasting};
use std::collections::HashSet;

/// System: Computes field-of-view for all entities with a Sight component.
///
/// Each tick, this system iterates entities with Sight, reads their Position,
/// and stores visible cell sets in `world.visible_cells`.
///
/// The FOV algorithm is auto-selected based on the map's topology type:
/// - `"square"` → [`RecursiveShadowcasting`]
/// - `"hex"` / `"province"` → [`BfsFovAlgorithm`]
///
/// Any [`FovAlgorithm`](crate::map::fov::FovAlgorithm) can be plugged in via
/// [`World::set_fov_algorithm`].
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

        // Auto-select the FOV algorithm based on map topology
        let desired = match map.topology_type() {
            "hex" | "province" => "bfs_flood_fill",
            _ => "recursive_shadowcasting",
        };
        if world.fov_algorithm.name() != desired {
            match desired {
                "bfs_flood_fill" => world.fov_algorithm = Box::new(BfsFovAlgorithm),
                _ => world.fov_algorithm = Box::new(RecursiveShadowcasting),
            }
        }

        // Collect-then-apply pattern to avoid borrow conflicts with world.components
        let mut results: Vec<(u32, HashSet<CellKey>)> = Vec::new();

        if let Some(sight_components) = world.components.get("Sight") {
            for (&entity, data) in sight_components.iter() {
                let range = data.get("range").and_then(|v| v.as_u64()).unwrap_or(8) as u32;

                if let Some(pos) = world
                    .get_component(entity, "Position")
                    .and_then(|comp| CellKey::from_position(comp))
                {
                    let visible =
                        world
                            .fov_algorithm()
                            .compute_fov(&pos, range, map.topology.as_ref());

                    let visible: HashSet<CellKey> = visible
                        .into_iter()
                        .filter(|cell| map.contains(cell))
                        .collect();

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
