use super::World;

impl World {
    pub fn modify_resource_amount(
        &mut self,
        entity_id: u32,
        kind: &str,
        delta: f64,
    ) -> Result<(), String> {
        let comp = self
            .components
            .get_mut("Resource")
            .and_then(|map| map.get_mut(&entity_id));
        if let Some(resource) = comp {
            if let Some(obj) = resource.as_object_mut() {
                if obj.get("kind").and_then(|v| v.as_str()) != Some(kind) {
                    return Err("Resource kind mismatch".to_string());
                }
                let amount = obj.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let new_amount = amount + delta;
                if new_amount < 0.0 {
                    return Err("Not enough resource".to_string());
                }
                obj.insert("amount".to_string(), serde_json::json!(new_amount));
                return Ok(());
            }
        }
        Err("Resource component not found".to_string())
    }

    pub fn modify_stockpile_resource(
        &mut self,
        entity_id: u32,
        kind: &str,
        delta: f64,
    ) -> Result<(), String> {
        let comp = self
            .components
            .get_mut("Stockpile")
            .and_then(|map| map.get_mut(&entity_id));
        if let Some(stockpile) = comp {
            if let Some(obj) = stockpile.as_object_mut() {
                if let Some(resources) = obj.get_mut("resources").and_then(|v| v.as_object_mut()) {
                    let current = resources.get(kind).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let new_amount = current + delta;
                    if new_amount < 0.0 {
                        return Err("Not enough resource".to_string());
                    }
                    resources.insert(kind.to_string(), serde_json::json!(new_amount));
                    return Ok(());
                }
            }
        }
        Err("Stockpile component not found".to_string())
    }
}
