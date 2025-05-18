use crate::ecs::components::position::{Position, PositionComponent};
use crate::ecs::system::System;
use crate::ecs::world::World;
use crate::map::CellKey;
use serde_json::json;

pub struct MoveAll {
    pub delta: MoveDelta,
}

pub enum MoveDelta {
    Square { dx: i32, dy: i32, dz: i32 },
    Hex { dq: i32, dr: i32, dz: i32 },
    Region { to_id: String },
}

impl System for MoveAll {
    fn name(&self) -> &'static str {
        "MoveAll"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let map = match &world.map {
            Some(map) => map,
            None => return,
        };

        if let Some(positions) = world.components.get_mut("PositionComponent") {
            for (_entity, value) in positions.iter_mut() {
                if let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value.clone())
                {
                    let new_pos = match (&pos_comp.pos, &self.delta) {
                        (Position::Square { x, y, z }, MoveDelta::Square { dx, dy, dz }) => {
                            let next = CellKey::Square {
                                x: x + dx,
                                y: y + dy,
                                z: z + dz,
                            };
                            if map.topology.contains(&next) {
                                Some(Position::Square {
                                    x: x + dx,
                                    y: y + dy,
                                    z: z + dz,
                                })
                            } else {
                                None
                            }
                        }
                        (Position::Hex { q, r, z }, MoveDelta::Hex { dq, dr, dz }) => {
                            let next = CellKey::Hex {
                                q: q + dq,
                                r: r + dr,
                                z: z + dz,
                            };
                            if map.topology.contains(&next) {
                                Some(Position::Hex {
                                    q: q + dq,
                                    r: r + dr,
                                    z: z + dz,
                                })
                            } else {
                                None
                            }
                        }
                        (Position::Region { .. }, MoveDelta::Region { to_id }) => {
                            let next = CellKey::Region { id: to_id.clone() };
                            if map.topology.contains(&next) {
                                Some(Position::Region { id: to_id.clone() })
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    if let Some(np) = new_pos {
                        pos_comp.pos = np;
                        *value = serde_json::to_value(&pos_comp).unwrap();
                    }
                }
            }
        }
    }
}

pub struct ProcessDeaths;
impl System for ProcessDeaths {
    fn name(&self) -> &'static str {
        "ProcessDeaths"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let mut to_process = Vec::new();

        // Collect entities with Health <= 0
        if let Some(healths) = world.components.get("Health") {
            for (&entity, value) in healths.iter() {
                if let Some(obj) = value.as_object() {
                    if let Some(current) = obj.get("current") {
                        if current.as_f64().unwrap_or(1.0) <= 0.0 {
                            to_process.push(entity);
                        }
                    }
                }
            }
        }

        for entity in to_process {
            // Remove Health component (if you want to simulate "dead" state)
            if let Some(healths) = world.components.get_mut("Health") {
                healths.remove(&entity);
            }

            // Add Corpse component
            let _ = world.set_component(entity, "Corpse", json!({}));

            // Add Decay component with default time_remaining (e.g., 5 ticks)
            let _ = world.set_component(entity, "Decay", json!({ "time_remaining": 5 }));
        }
    }
}

/// System: Damages all entities with a Health component by a given amount.
pub struct DamageAll {
    pub amount: f32,
}
impl System for DamageAll {
    fn name(&self) -> &'static str {
        "DamageAll"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        if let Some(healths) = world.components.get_mut("Health") {
            for (_entity, value) in healths.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(current) = obj.get_mut("current") {
                        if let Some(cur_val) = current.as_f64() {
                            let new_val = (cur_val - self.amount as f64).max(0.0);
                            *current = json!(new_val);
                        }
                    }
                }
            }
        }
    }
}

/// System: Processes decay for entities with a Decay component.
pub struct ProcessDecay;
impl System for ProcessDecay {
    fn name(&self) -> &'static str {
        "ProcessDecay"
    }
    fn run(&mut self, world: &mut World, _lua: Option<&mlua::Lua>) {
        let mut to_despawn_entities = Vec::new();
        if let Some(decays) = world.components.get_mut("Decay") {
            for (&entity, value) in decays.iter_mut() {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(time_remaining) = obj.get_mut("time_remaining") {
                        if let Some(t) = time_remaining.as_u64() {
                            if t <= 1 {
                                to_despawn_entities.push(entity);
                            } else {
                                *time_remaining = json!(t - 1);
                            }
                        }
                    }
                }
            }
        }
        for entity in to_despawn_entities {
            world.despawn_entity(entity);
        }
    }
}
