use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MapCell {
    pub id: String,
    pub neighbors: HashSet<String>,
    pub topology: String,
}

pub struct Map {
    pub cells: HashMap<String, MapCell>,
    pub topology: String,
}

impl Map {
    pub fn new(topology: &str) -> Self {
        Self {
            cells: HashMap::new(),
            topology: topology.to_string(),
        }
    }

    pub fn add_cell(&mut self, id: &str) {
        let cell = MapCell {
            id: id.to_string(),
            neighbors: HashSet::new(),
            topology: self.topology.clone(),
        };
        self.cells.insert(id.to_string(), cell);
    }

    pub fn add_neighbor(&mut self, id: &str, neighbor_id: &str) {
        if let Some(cell) = self.cells.get_mut(id) {
            cell.neighbors.insert(neighbor_id.to_string());
        }
    }

    pub fn neighbors(&self, id: &str) -> Option<&HashSet<String>> {
        self.cells.get(id).map(|c| &c.neighbors)
    }
}
