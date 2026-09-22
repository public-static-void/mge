use crate::ecs::system::System;
use crate::ecs::world::{Season, WeatherCondition, World};
use crate::map::Map;
use crate::map::cell_key::CellKey;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::HashSet;

/// Persistent global ambient temperature state, stored on [`World`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureState {
    /// Current global ambient temperature in °C.
    pub ambient: f64,
    /// Script override; when set, derivation is skipped and ambient holds this value.
    pub manual_override: Option<f64>,
}

impl Default for TemperatureState {
    fn default() -> Self {
        Self {
            ambient: 15.0,
            manual_override: None,
        }
    }
}

/// Humidity modifier band in °C: `H = (humidity - 0.5) * 2 * HUMIDITY_BAND`.
/// Bound here; applied by the humidity/pressure increment.
pub const HUMIDITY_BAND: f64 = 3.0;
/// Pressure modifier band in °C: `P = clamp((pressure - 1013.0) * 0.05, ±PRESSURE_BAND)`.
/// Bound here; applied by the humidity/pressure increment.
pub const PRESSURE_BAND: f64 = 2.0;
/// Diffusion rate for the single per-tick relaxation pass over the transient
/// cell temperature map. Bound here; applied by the diffusion increment.
pub const DIFFUSION_RATE: f64 = 0.2;
/// Default relative humidity (`[0.0, 1.0]`) for saves predating the field.
pub const DEFAULT_HUMIDITY: f64 = 0.5;
/// Default atmospheric pressure in hPa for saves predating the field.
pub const DEFAULT_PRESSURE: f64 = 1013.0;

/// System: Derives the global ambient temperature each tick, recomputes the
/// transient per-cell temperature map, and exchanges heat with every `Body`
/// part in place.
///
/// Ambient comes from [`compute_ambient_temperature`] applied to the same-tick
/// `Season + WeatherState + TimeOfDay` output (this system runs immediately
/// after `WeatherSystem`), unless a script override holds it fixed. The
/// transient map is then seeded from ambient, heated by `HeatSource`
/// entities, and relaxed in one synchronous diffusion pass. Each part drifts
/// toward its entity cell temperature at a rate scaled by its stored
/// `insulation`, threshold crossings emit `cold_stress` / `heat_stress`
/// events, and extreme exposure queues `PendingDamage` for
/// `BodyPartDamageSystem` on the following tick.
///
/// `insulation` is read-only input here: [`EquipmentEffectAggregationSystem`]
/// is its sole writer.
pub struct TemperatureSystem;

impl System for TemperatureSystem {
    fn name(&self) -> &'static str {
        "TemperatureSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &["WeatherSystem"]
    }

    fn run(&mut self, world: &mut World) {
        // Guard: no map loaded → no-op (mirrors WeatherSystem).
        if world.map.is_none() {
            return;
        }

        if let Some(override_value) = world.temperature.manual_override {
            world.temperature.ambient = override_value;
        } else {
            let season = Season::from_day(world.time_of_day.day);
            world.temperature.ambient = compute_ambient_temperature(
                season,
                world.weather.condition,
                world.weather.intensity,
                world.time_of_day.hour,
                world.time_of_day.minute,
                world.weather.humidity,
                world.weather.pressure,
            );
        }
        let ambient = world.temperature.ambient;

        // Recompute the transient per-cell map (seed → heat sources → one
        // relaxation pass) before per-part drift reads it.
        recompute_temperature_map(world, ambient);

        // Take the stress flags locally so part processing never holds a
        // borrow on `world` while reading/writing components; prune flags for
        // entities that no longer carry a Body.
        let mut stressed = std::mem::take(&mut world.temperature_stress);
        {
            let bodies: HashSet<u32> = world
                .get_entities_with_component("Body")
                .into_iter()
                .collect();
            stressed.retain(|(entity, _)| bodies.contains(entity));
        }

        // Sorted entity iteration keeps multi-entity ticks deterministic.
        let mut entities = world.get_entities_with_component("Body");
        entities.sort_unstable();

        for entity in entities {
            let Some(body) = world.get_component(entity, "Body").cloned() else {
                continue;
            };
            let mut body = body;
            let mut events: Vec<(bool, String, f64, f64)> = Vec::new();
            let mut damage_parts: Vec<String> = Vec::new();
            if let Some(parts) = body.get_mut("parts").and_then(|v| v.as_array_mut()) {
                let target = cell_temperature_for(world, entity, ambient);
                process_parts(
                    parts,
                    target,
                    entity,
                    &mut stressed,
                    &mut events,
                    &mut damage_parts,
                );
            }
            if world.set_component(entity, "Body", body).is_err() {
                continue;
            }
            for (is_cold, part, temperature, ideal) in events {
                let event_name = if is_cold {
                    "cold_stress"
                } else {
                    "heat_stress"
                };
                let _ = world.send_event(
                    event_name,
                    json!({
                        "entity": entity,
                        "part": part,
                        "temperature": temperature,
                        "ideal": ideal,
                    }),
                );
            }
            for part in damage_parts {
                world.damage_entity_part(entity, &part, 1.0);
            }
        }

        world.temperature_stress = stressed;
    }
}

/// Compute the global ambient temperature in °C from season, weather, time,
/// humidity, and pressure.
///
/// `ambient = season_base + weather_delta + diurnal + humidity_delta +
/// pressure_delta`, clamped to `[-60, 60]`, with `diurnal = 5·cos(2π·(t−14)/24)`
/// peaking at 14:00, `humidity_delta = (humidity − 0.5) * 6.0` (band ±3 °C),
/// and `pressure_delta = clamp((pressure − 1013.0) * 0.05, ±2.0)`. At the
/// canonical neutral point (`humidity = 0.5`, `pressure = 1013.0`) both
/// modifiers are exactly zero, reproducing the legacy ambient value. Pure
/// function: no RNG, no wall-clock reads, so identical inputs always yield
/// identical outputs.
pub fn compute_ambient_temperature(
    season: Season,
    condition: WeatherCondition,
    intensity: f64,
    hour: u8,
    minute: u8,
    humidity: f64,
    pressure: f64,
) -> f64 {
    let season_base = match season {
        Season::Winter => -10.0,
        Season::Spring => 10.0,
        Season::Summer => 25.0,
        Season::Autumn => 12.0,
    };
    let weather_delta = match condition {
        WeatherCondition::Clear => 0.0,
        WeatherCondition::Cloudy => -2.0,
        WeatherCondition::Rain => -5.0 * intensity,
        WeatherCondition::Snow => -12.0 * intensity,
        WeatherCondition::Storm => -8.0 * intensity,
        WeatherCondition::Fog => -3.0 * intensity,
    };
    let t = f64::from(hour) + f64::from(minute) / 60.0;
    let diurnal = 5.0 * (2.0 * std::f64::consts::PI * (t - 14.0) / 24.0).cos();
    let humidity_delta = (humidity - DEFAULT_HUMIDITY) * 2.0 * HUMIDITY_BAND;
    let pressure_delta =
        ((pressure - DEFAULT_PRESSURE) * 0.05).clamp(-PRESSURE_BAND, PRESSURE_BAND);
    (season_base + weather_delta + diurnal + humidity_delta + pressure_delta).clamp(-60.0, 60.0)
}

/// Deterministic sort key for map cells (canonical JSON form).
fn cell_sort_key(cell: &CellKey) -> String {
    serde_json::to_string(cell).unwrap_or_default()
}

/// Opaque cells (`transparent: false` in metadata) block heat exchange — same
/// rule as [`BfsFovAlgorithm`](crate::map::fov::BfsFovAlgorithm) and noise
/// propagation. Cells without metadata default to transparent.
fn is_opaque_cell(map: &Map, cell: &CellKey) -> bool {
    map.get_cell_metadata(cell)
        .and_then(|m| m.get("transparent"))
        .and_then(|v| v.as_bool())
        .map(|t| !t)
        .unwrap_or(false)
}

/// Recompute the transient per-cell temperature map for this tick: seed every
/// map cell with ambient, add each active `HeatSource` `{intensity}` at its
/// entity cell, then run exactly one synchronous relaxation pass `T_new(c) =
/// T(c) + D * (avg(neighbors) - T(c))` with `D = DIFFUSION_RATE` over
/// transparent neighbors only. Opaque cells neither give nor receive.
///
/// No per-emitter flood-fill (NFR002): sources touch only their own cell, so
/// the pass is O(cells) with sorted cell order and the reused
/// `temperature_scratch` buffer for determinism.
fn recompute_temperature_map(world: &mut World, ambient: f64) {
    let Some(map) = world.map.as_ref() else {
        world.temperature_map.clear();
        return;
    };
    let mut cells = map.all_cells();
    cells.sort_by_key(cell_sort_key);

    let seeded = &mut world.temperature_map;
    seeded.clear();
    seeded.reserve(cells.len());
    for cell in &cells {
        seeded.insert(cell.clone(), ambient);
    }

    if let Some(sources) = world.components.get("HeatSource") {
        let mut emitters: Vec<u32> = sources.keys().copied().collect();
        emitters.sort_unstable();
        for entity in emitters {
            let Some(data) = sources.get(&entity) else {
                continue;
            };
            let active = data.get("active").and_then(|v| v.as_bool()).unwrap_or(true);
            if !active {
                continue;
            }
            let intensity = data.get("intensity").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if !intensity.is_finite() || intensity == 0.0 {
                continue;
            }
            let Some(cell) = world
                .components
                .get("Position")
                .and_then(|positions| positions.get(&entity))
                .and_then(CellKey::from_position)
            else {
                continue;
            };
            if !map.contains(&cell) {
                continue;
            }
            if let Some(slot) = seeded.get_mut(&cell) {
                *slot += intensity;
            }
        }
    }

    let scratch = &mut world.temperature_scratch;
    scratch.clear();
    scratch.reserve(cells.len());
    for cell in &cells {
        let current = seeded.get(cell).copied().unwrap_or(ambient);
        if is_opaque_cell(map, cell) {
            scratch.insert(cell.clone(), current);
            continue;
        }
        let mut sum = 0.0;
        let mut count = 0u32;
        for neighbor in map.neighbors(cell) {
            if !map.contains(&neighbor) || is_opaque_cell(map, &neighbor) {
                continue;
            }
            if let Some(&t) = seeded.get(&neighbor) {
                sum += t;
                count += 1;
            }
        }
        let next = if count == 0 {
            current
        } else {
            current + DIFFUSION_RATE * (sum / f64::from(count) - current)
        };
        scratch.insert(cell.clone(), next);
    }
    std::mem::swap(&mut world.temperature_map, &mut world.temperature_scratch);
}

/// Drift target for one entity: the transient map value at its cell when the
/// map is populated, else global ambient.
fn cell_temperature_for(world: &World, entity: u32, ambient: f64) -> f64 {
    if world.temperature_map.is_empty() {
        return ambient;
    }
    let Some(cell) = world
        .get_component(entity, "Position")
        .and_then(CellKey::from_position)
    else {
        return ambient;
    };
    world.temperature_map.get(&cell).copied().unwrap_or(ambient)
}

/// Advance every part (including nested `children`) toward the drift target.
///
/// The target is the entity cell temperature from the transient diffusion map
/// (or global ambient when the map is empty). Null `temperature` initializes
/// to `ideal_temperature` (or the target when the ideal is also null) with no
/// event; null `insulation` counts as `0.0`. `heat_loss` records
/// `temp_before − temp_after`. Stress events fire at most once per part per
/// crossing and re-arm inside the ±15 band; deviation of ±25 additionally
/// queues `1.0` damage for the named part.
#[allow(clippy::too_many_arguments)]
fn process_parts(
    parts: &mut [JsonValue],
    target: f64,
    entity: u32,
    stressed: &mut HashSet<(u32, String)>,
    events: &mut Vec<(bool, String, f64, f64)>,
    damage_parts: &mut Vec<String>,
) {
    for part in parts.iter_mut() {
        let name = part
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("")
            .to_string();
        let ideal = part.get("ideal_temperature").and_then(|v| v.as_f64());
        // Static read-only input: insulation is never written back.
        let insulation = part
            .get("insulation")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        if let Some(current) = part.get("temperature").and_then(|v| v.as_f64()) {
            let k = 0.05 / (1.0 + insulation);
            let next = current + (target - current) * k;
            part["temperature"] = json!(next);
            part["heat_loss"] = json!(current - next);

            if !name.is_empty()
                && let Some(ideal_value) = ideal
            {
                let key = (entity, name.clone());
                if next <= ideal_value - 15.0 {
                    if !stressed.contains(&key) {
                        stressed.insert(key);
                        events.push((true, name.clone(), next, ideal_value));
                    }
                } else if next >= ideal_value + 15.0 {
                    if !stressed.contains(&key) {
                        stressed.insert(key);
                        events.push((false, name.clone(), next, ideal_value));
                    }
                } else {
                    stressed.remove(&key);
                }
                if next <= ideal_value - 25.0 || next >= ideal_value + 25.0 {
                    damage_parts.push(name.clone());
                }
            }
        } else {
            part["temperature"] = json!(ideal.unwrap_or(target));
            part["heat_loss"] = json!(0.0);
        }

        if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
            process_parts(children, target, entity, stressed, events, damage_parts);
        }
    }
}

impl World {
    /// Current global ambient temperature in °C.
    pub fn get_temperature(&self) -> f64 {
        self.temperature.ambient
    }

    /// Hold ambient at `ambient` °C: clamps to `[-60, 60]`, sets the manual
    /// override (derivation is skipped while it is set), applies it
    /// immediately, and emits a `temperature_changed` event.
    pub fn set_temperature(&mut self, ambient: f64) {
        let clamped = ambient.clamp(-60.0, 60.0);
        let old_ambient = self.temperature.ambient;
        self.temperature.ambient = clamped;
        self.temperature.manual_override = Some(clamped);
        let _ = self.send_event(
            "temperature_changed",
            json!({
                "old_ambient": old_ambient,
                "new_ambient": clamped,
            }),
        );
    }

    /// Per-cell temperature from the transient diffusion map.
    /// Falls back to global ambient when the map is empty or the cell is absent.
    pub fn get_cell_temperature(&self, x: i32, y: i32, z: i32) -> f64 {
        let cell = CellKey::Square { x, y, z };
        self.temperature_map
            .get(&cell)
            .copied()
            .unwrap_or(self.temperature.ambient)
    }
}
