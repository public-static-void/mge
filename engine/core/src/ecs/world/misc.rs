use super::World;
use crate::ecs::components::position::{Position, PositionComponent};

impl World {
    pub fn move_entity(&mut self, entity: u32, dx: f32, dy: f32) {
        if let Some(value) = self.get_component(entity, "PositionComponent").cloned() {
            if let Ok(mut pos_comp) = serde_json::from_value::<PositionComponent>(value) {
                if let Position::Square { x, y, .. } = &mut pos_comp.pos {
                    *x += dx as i32;
                    *y += dy as i32;
                }
                let _ = self.set_component(
                    entity,
                    "PositionComponent",
                    serde_json::to_value(&pos_comp).unwrap(),
                );
            }
        }
    }

    pub fn damage_entity(&mut self, entity: u32, amount: f32) {
        if let Some(healths) = self.components.get_mut("Health") {
            if let Some(value) = healths.get_mut(&entity) {
                if let Some(obj) = value.as_object_mut() {
                    if let Some(current) = obj.get_mut("current") {
                        if let Some(cur_val) = current.as_f64() {
                            *current = serde_json::json!((cur_val - amount as f64).max(0.0));
                        }
                    }
                }
            }
        }
    }

    pub fn is_entity_alive(&self, entity: u32) -> bool {
        if let Some(health) = self.get_component(entity, "Health") {
            health
                .get("current")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                > 0.0
        } else {
            false
        }
    }

    pub fn count_entities_with_type(&self, type_str: &str) -> usize {
        self.get_entities_with_component("Type")
            .into_iter()
            .filter(|&id| {
                self.get_component(id, "Type")
                    .and_then(|v| v.get("kind"))
                    .and_then(|k| k.as_str())
                    .map(|k| k == type_str)
                    .unwrap_or(false)
            })
            .count()
    }
}
