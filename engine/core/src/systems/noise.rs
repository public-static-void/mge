use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::cell_key::CellKey;
use std::collections::{HashMap, VecDeque};

/// Shared fallback defaults for noise emission, read by both the core
/// [`NoiseSystem`] and the WASM bridge `tick_noise` mirror so the two sides
/// can never drift apart. Follows the `DungeonConfig::DEFAULT_*` precedent:
/// one definition per domain, no central `Defaults` object.
pub mod noise_defaults {
    /// Emitters without an `active` flag are processed.
    pub const ACTIVE: bool = true;
    /// Fallback `NoiseEmitter.intensity` when the field is absent.
    pub const INTENSITY: f64 = 1.0;
    /// Fallback `NoiseEmitter.radius` when the field is absent.
    pub const RADIUS: u64 = 5;
    /// Fallback `Stealth.noise_modifier` when the field is absent.
    pub const NOISE_MODIFIER: f64 = 1.0;
}

/// System: Propagates noise from entities with a NoiseEmitter component.
///
/// Uses BFS flood-fill from each emitter position with linear falloff:
/// `noise_at_cell = intensity * (radius - distance) / radius`.
///
/// Opaque cells (`transparent: false`) block propagation — same rule as
/// [`BfsFovAlgorithm`](crate::map::fov::BfsFovAlgorithm).  Multiple emitters
/// contributing to the same cell use max-aggregation.
///
/// Follows the collect-then-apply pattern from [`FovUpdateSystem`](super::fov).
/// The resulting `noise_map` is applied via [`World::set_noise_map`] each tick.
pub struct NoiseSystem;

impl System for NoiseSystem {
    fn name(&self) -> &'static str {
        "NoiseSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["FovUpdateSystem"]
    }

    fn run(&mut self, world: &mut World) {
        let map = match &world.map {
            Some(m) => m,
            None => return,
        };

        let mut noise_map: HashMap<CellKey, f64> = HashMap::new();

        if let Some(emitters) = world.components.get("NoiseEmitter") {
            for (&entity, data) in emitters.iter() {
                // Skip inactive emitters (R010)
                let active = data
                    .get("active")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(noise_defaults::ACTIVE);
                if !active {
                    continue;
                }

                let intensity = data
                    .get("intensity")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(noise_defaults::INTENSITY);
                let radius = data
                    .get("radius")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(noise_defaults::RADIUS) as i32;

                // Apply Stealth modifier: effective intensity = intensity * noise_modifier (R004)
                let effective_intensity =
                    if let Some(stealth) = world.get_component(entity, "Stealth") {
                        let modifier = stealth
                            .get("noise_modifier")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(noise_defaults::NOISE_MODIFIER);
                        intensity * modifier
                    } else {
                        intensity
                    };

                // Skip emitters that end up with zero effective intensity
                if effective_intensity <= 0.0 {
                    continue;
                }

                // Get emitter position
                let origin = match world
                    .get_component(entity, "Position")
                    .and_then(CellKey::from_position)
                {
                    Some(pos) => pos,
                    None => continue,
                };

                if !map.contains(&origin) {
                    continue;
                }

                // BFS flood-fill with linear falloff (R006, R007)
                let propagated = bfs_noise_propagation(
                    &origin,
                    effective_intensity,
                    radius,
                    map.topology.as_ref(),
                );

                // Max-aggregation: take the max at each cell (R008)
                for (cell, noise) in propagated {
                    let entry = noise_map.entry(cell).or_insert(0.0);
                    if noise > *entry {
                        *entry = noise;
                    }
                }
            }
        }

        world.set_noise_map(noise_map);
    }
}

/// Shared noise propagation kernel used by both the core [`NoiseSystem`] and
/// the WASM bridge `tick_noise` mirror.
///
/// The kernel owns the propagation shape — BFS flood-fill with linear
/// falloff (`intensity * (radius - distance) / radius`), origin keeping full
/// intensity, opaque cells receiving noise without relaying it further — and
/// is parameterized over neighbor-lookup and opacity closures so each side
/// keeps its own transport: the core side passes [`MapTopology`] accessors,
/// the WASM side passes JSON-adjacency adapters without forcing the typed
/// topology trait across the serialization boundary.
///
/// [`MapTopology`]: crate::map::topology::MapTopology
pub fn propagate_noise_kernel(
    origin: &CellKey,
    intensity: f64,
    radius: i32,
    neighbors_of: impl Fn(&CellKey) -> Vec<CellKey>,
    is_opaque: impl Fn(&CellKey) -> bool,
) -> HashMap<CellKey, f64> {
    let mut result: HashMap<CellKey, f64> = HashMap::new();
    let mut queue: VecDeque<(CellKey, i32)> = VecDeque::new();
    let mut visited: std::collections::HashSet<CellKey> = std::collections::HashSet::new();

    result.insert(origin.clone(), intensity);
    visited.insert(origin.clone());
    queue.push_back((origin.clone(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= radius {
            continue;
        }

        for neighbor in neighbors_of(&current) {
            if visited.contains(&neighbor) {
                continue;
            }

            visited.insert(neighbor.clone());
            let distance = depth + 1;

            // Linear falloff: intensity * (radius - distance) / radius
            let noise = intensity * (radius - distance) as f64 / radius as f64;
            result.insert(neighbor.clone(), noise);

            // Opaque cells receive noise but block further propagation.
            if !is_opaque(&neighbor) {
                queue.push_back((neighbor, distance));
            }
        }
    }

    result
}

/// BFS noise propagation from an origin cell with linear falloff.
///
/// Returns a map of cells to their noise level.  Opaque cells block
/// propagation — the cell itself receives noise but neighbors beyond it
/// are not reached.  The origin cell always receives full intensity.
///
/// Thin typed-topology adapter over [`propagate_noise_kernel`].
fn bfs_noise_propagation(
    origin: &CellKey,
    intensity: f64,
    radius: i32,
    topology: &dyn crate::map::topology::MapTopology,
) -> HashMap<CellKey, f64> {
    propagate_noise_kernel(
        origin,
        intensity,
        radius,
        |cell| {
            topology
                .neighbors(cell)
                .into_iter()
                .filter(|neighbor| topology.contains(neighbor))
                .collect()
        },
        |cell| {
            topology
                .get_cell_metadata(cell)
                .and_then(|m| m.get("transparent"))
                .and_then(|v| v.as_bool())
                .map(|t| !t)
                .unwrap_or(false)
        },
    )
}
