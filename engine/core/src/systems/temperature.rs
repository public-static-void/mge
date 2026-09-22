use crate::ecs::system::System;
use crate::ecs::world::{Season, WeatherCondition, World};
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

/// System: Derives the global ambient temperature each tick and exchanges heat
/// with every `Body` part in place.
///
/// Ambient comes from [`compute_ambient_temperature`] applied to the same-tick
/// `Season + WeatherState + TimeOfDay` output (this system runs immediately
/// after `WeatherSystem`), unless a script override holds it fixed. Each part
/// drifts toward ambient at a rate scaled by its stored `insulation`, threshold
/// crossings emit `cold_stress` / `heat_stress` events, and extreme exposure
/// queues `PendingDamage` for `BodyPartDamageSystem` on the following tick.
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
            );
        }
        let ambient = world.temperature.ambient;

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
                process_parts(
                    parts,
                    ambient,
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

/// Compute the global ambient temperature in °C from season, weather, and time.
///
/// `ambient = season_base + weather_delta + diurnal`, clamped to `[-60, 60]`,
/// with `diurnal = 5·cos(2π·(t−14)/24)` peaking at 14:00. Pure function: no RNG,
/// no wall-clock reads, so identical inputs always yield identical outputs.
pub fn compute_ambient_temperature(
    season: Season,
    condition: WeatherCondition,
    intensity: f64,
    hour: u8,
    minute: u8,
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
    (season_base + weather_delta + diurnal).clamp(-60.0, 60.0)
}

/// Advance every part (including nested `children`) toward ambient.
///
/// Null `temperature` initializes to `ideal_temperature` (or ambient when the
/// ideal is also null) with no event; null `insulation` counts as `0.0`.
/// `heat_loss` records `temp_before − temp_after`. Stress events fire at most
/// once per part per crossing and re-arm inside the ±15 band; deviation of ±25
/// additionally queues `1.0` damage for the named part.
#[allow(clippy::too_many_arguments)]
fn process_parts(
    parts: &mut [JsonValue],
    ambient: f64,
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
            let next = current + (ambient - current) * k;
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
            part["temperature"] = json!(ideal.unwrap_or(ambient));
            part["heat_loss"] = json!(0.0);
        }

        if let Some(children) = part.get_mut("children").and_then(|v| v.as_array_mut()) {
            process_parts(children, ambient, entity, stressed, events, damage_parts);
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
}
