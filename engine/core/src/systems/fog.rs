use crate::ecs::system::System;
use crate::ecs::world::World;

/// Visibility state enum for fog-of-war.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisibilityState {
    /// Cell has never been observed by this entity.
    Unexplored = 0,
    /// Cell has been observed before but is not currently in FOV.
    Explored = 1,
    /// Cell is currently in this entity's field-of-view.
    Visible = 2,
}

/// System: Updates fog-of-war state by merging visible cells into explored cells.
///
/// Runs AFTER [`FovUpdateSystem`] each tick. For every entity that has a non-empty
/// `visible_cells` entry, the system merges those cells into the entity's `explored_cells`
/// set (set union). Entities with no `visible_cells` entry are skipped.
pub struct FogUpdateSystem;

impl System for FogUpdateSystem {
    fn name(&self) -> &'static str {
        "FogUpdateSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["FovUpdateSystem"]
    }

    fn run(&mut self, world: &mut World) {
        // Collect-then-apply pattern to avoid borrow conflicts with world internals.
        let mut updates: Vec<(u32, Vec<crate::map::cell_key::CellKey>)> = Vec::new();

        for (&entity, visible) in &world.visible_cells {
            if visible.is_empty() {
                continue;
            }
            // Collect visible cells to merge
            let cells: Vec<_> = visible.iter().cloned().collect();
            updates.push((entity, cells));
        }

        // Apply collected updates
        for (entity, cells) in updates {
            let explored = world.explored_cells.entry(entity).or_default();
            explored.extend(cells);
        }
    }
}
