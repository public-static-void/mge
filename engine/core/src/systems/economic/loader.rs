use super::recipe::Recipe;
use std::fs;
use std::path::Path;

pub fn load_recipes_from_dir<P: AsRef<Path>>(dir: P) -> Vec<Recipe> {
    let mut recipes = Vec::new();
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return recipes,
    };
    for entry in entries {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let data = fs::read_to_string(&path).expect("Failed to read recipe file");
            let recipe: Recipe = serde_json::from_str(&data).expect("Failed to parse recipe file");
            recipes.push(recipe);
        }
    }
    recipes
}
