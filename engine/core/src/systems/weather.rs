use crate::ecs::system::System;
use crate::ecs::world::{Season, WeatherCondition, World};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::SmallRng;

/// System: Advances global weather state each tick.
///
/// Weather is world-level deterministic state. Each tick the system decrements
/// the remaining duration and, when it expires, transitions to a new condition
/// using season-weighted probabilities. The resulting `visibility_modifier` is
/// written to `world.visibility_modifier` for downstream consumers like
/// `FovUpdateSystem`.
pub struct WeatherSystem;

impl System for WeatherSystem {
    fn name(&self) -> &'static str {
        "WeatherSystem"
    }

    fn dependencies(&self) -> &'static [&'static str] {
        &[]
    }

    fn run(&mut self, world: &mut World) {
        // Guard: no map loaded → no-op (consistent with NoiseSystem pattern).
        if world.map.is_none() {
            return;
        }

        // Decrement remaining duration (saturating)
        world.weather.duration_remaining = world.weather.duration_remaining.saturating_sub(1);

        if world.weather.duration_remaining == 0 {
            let old_condition = world.weather.condition;

            let season = Season::from_day(world.time_of_day.day);

            // Deterministic RNG derived from persisted seed (R013 pattern).
            let mut rng = SmallRng::from_seed(world.weather.rng_state);

            let (new_condition, new_intensity, new_duration) =
                generate_weather_transition(old_condition, season, &mut rng);

            // Persist updated RNG seed for next tick's determinism.
            rng.fill(&mut world.weather.rng_state);

            world.weather.condition = new_condition;
            world.weather.intensity = new_intensity;
            world.weather.duration_remaining = new_duration;

            // Emit transition event (OQ4: forced transitions also emit).
            let _ = world.send_event(
                "weather_changed",
                serde_json::json!({
                    "old_condition": old_condition.as_str(),
                    "new_condition": new_condition.as_str(),
                    "intensity": new_intensity,
                }),
            );
        }

        // Recompute visibility modifier from current weather state.
        world.visibility_modifier =
            compute_visibility_modifier(world.weather.condition, world.weather.intensity);
    }
}

/// Compute the visibility modifier from weather condition and intensity.
///
/// Returns 1.0 (no reduction) for Clear, scaling down with intensity for
/// precipitation and fog types per R008.
pub fn compute_visibility_modifier(condition: WeatherCondition, intensity: f64) -> f64 {
    match condition {
        WeatherCondition::Clear => 1.0,
        WeatherCondition::Cloudy => 0.9,
        WeatherCondition::Rain => 0.7 * intensity,
        WeatherCondition::Snow => 0.6 * intensity,
        WeatherCondition::Storm => 0.5 * intensity,
        WeatherCondition::Fog => 0.4 * intensity,
    }
}

/// Generate a weather transition using season-weighted probabilities.
///
/// Returns (new_condition, intensity, duration_remaining).
/// Intensity is 0.0–1.0, duration is 50–200 ticks.
fn generate_weather_transition(
    current: WeatherCondition,
    season: Season,
    rng: &mut SmallRng,
) -> (WeatherCondition, f64, u32) {
    let candidates = season_candidates(current, season);

    let total_weight: u32 = candidates.iter().map(|(_, w)| w).sum();
    let roll = rng.random_range(0..total_weight);

    let mut cumulative = 0u32;
    let mut chosen = candidates[0].0;
    for (cond, weight) in &candidates {
        cumulative += weight;
        if roll < cumulative {
            chosen = *cond;
            break;
        }
    }

    let intensity = rng.random_range(0.0..=1.0);
    let duration = rng.random_range(50..=200);

    (chosen, intensity, duration)
}

/// Build candidate list (condition, weight) based on current condition
/// attractor rules and season weights (R006, R-03).
fn season_candidates(current: WeatherCondition, season: Season) -> Vec<(WeatherCondition, u32)> {
    match current {
        // Snow attractor: only → Storm or Clear (R006)
        WeatherCondition::Snow => match season {
            Season::Winter => vec![(WeatherCondition::Storm, 40), (WeatherCondition::Clear, 60)],
            _ => vec![(WeatherCondition::Storm, 30), (WeatherCondition::Clear, 70)],
        },
        // Fog attractor: only → Clear or Cloudy (R006)
        WeatherCondition::Fog => vec![
            (WeatherCondition::Clear, 50),
            (WeatherCondition::Cloudy, 50),
        ],
        // Cloudy attractor: favor clearing or intensifying — exclude Cloudy itself (R-03)
        WeatherCondition::Cloudy => match season {
            Season::Spring => vec![
                (WeatherCondition::Clear, 35),
                (WeatherCondition::Rain, 35),
                (WeatherCondition::Fog, 15),
                (WeatherCondition::Snow, 15),
            ],
            Season::Summer => vec![
                (WeatherCondition::Clear, 40),
                (WeatherCondition::Rain, 25),
                (WeatherCondition::Storm, 20),
                (WeatherCondition::Fog, 15),
            ],
            Season::Autumn => vec![
                (WeatherCondition::Clear, 25),
                (WeatherCondition::Rain, 35),
                (WeatherCondition::Fog, 25),
                (WeatherCondition::Snow, 15),
            ],
            Season::Winter => vec![
                (WeatherCondition::Clear, 30),
                (WeatherCondition::Snow, 35),
                (WeatherCondition::Storm, 20),
                (WeatherCondition::Fog, 15),
            ],
        },
        // General transitions: season-weighted base probabilities (R006)
        _ => match season {
            Season::Spring => vec![
                (WeatherCondition::Clear, 40),
                (WeatherCondition::Cloudy, 25),
                (WeatherCondition::Rain, 25),
                (WeatherCondition::Fog, 10),
            ],
            Season::Summer => vec![
                (WeatherCondition::Clear, 50),
                (WeatherCondition::Cloudy, 20),
                (WeatherCondition::Storm, 15),
                (WeatherCondition::Rain, 15),
            ],
            Season::Autumn => vec![
                (WeatherCondition::Cloudy, 30),
                (WeatherCondition::Rain, 30),
                (WeatherCondition::Fog, 20),
                (WeatherCondition::Clear, 20),
            ],
            Season::Winter => vec![
                (WeatherCondition::Cloudy, 25),
                (WeatherCondition::Snow, 30),
                (WeatherCondition::Clear, 25),
                (WeatherCondition::Storm, 20),
            ],
        },
    }
}
