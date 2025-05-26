use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::Any;
use std::collections::{HashMap, HashSet};

type CellSet = HashSet<CellKey>;

/// Trait for any map topology (tile, hex, region, etc.)
pub trait MapTopology: Send + Sync {
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey>;
    fn contains(&self, cell: &CellKey) -> bool;
    fn all_cells(&self) -> Vec<CellKey>;
    fn topology_type(&self) -> &'static str;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    // Cell metadata API
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value);
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CellKey {
    Square { x: i32, y: i32, z: i32 },
    Hex { q: i32, r: i32, z: i32 },
    Region { id: String },
}

/// Square grid with z-levels (Dwarf Fortress style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquareGridMap {
    pub cells: HashMap<CellKey, CellSet>,
    pub cell_metadata: HashMap<CellKey, Value>,
}

impl SquareGridMap {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }
    pub fn add_cell(&mut self, x: i32, y: i32, z: i32) {
        self.cells.entry(CellKey::Square { x, y, z }).or_default();
    }
    pub fn add_neighbor(&mut self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        self.cells
            .entry(CellKey::Square {
                x: from.0,
                y: from.1,
                z: from.2,
            })
            .or_default()
            .insert(CellKey::Square {
                x: to.0,
                y: to.1,
                z: to.2,
            });
    }
}

impl MapTopology for SquareGridMap {
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        if let CellKey::Square { .. } = cell {
            self.cells
                .get(cell)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            vec![]
        }
    }
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Square { .. } if self.cells.contains_key(cell))
    }
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells.keys().cloned().collect()
    }
    fn topology_type(&self) -> &'static str {
        "square"
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        self.cell_metadata.insert(cell.clone(), data);
    }
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        self.cell_metadata.get(cell)
    }
}

impl Default for SquareGridMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Hex grid with z-levels (Panzer General style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HexGridMap {
    pub cells: HashMap<CellKey, CellSet>,
    pub cell_metadata: HashMap<CellKey, Value>,
}

impl HexGridMap {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }
    pub fn add_cell(&mut self, q: i32, r: i32, z: i32) {
        self.cells.entry(CellKey::Hex { q, r, z }).or_default();
    }
    pub fn add_neighbor(&mut self, from: (i32, i32, i32), to: (i32, i32, i32)) {
        self.cells
            .entry(CellKey::Hex {
                q: from.0,
                r: from.1,
                z: from.2,
            })
            .or_default()
            .insert(CellKey::Hex {
                q: to.0,
                r: to.1,
                z: to.2,
            });
    }
}

impl Default for HexGridMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MapTopology for HexGridMap {
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        if let CellKey::Hex { .. } = cell {
            self.cells
                .get(cell)
                .map(|set| set.iter().cloned().collect())
                .unwrap_or_default()
        } else {
            vec![]
        }
    }
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Hex { .. } if self.cells.contains_key(cell))
    }
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells.keys().cloned().collect()
    }
    fn topology_type(&self) -> &'static str {
        "hex"
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        self.cell_metadata.insert(cell.clone(), data);
    }
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        self.cell_metadata.get(cell)
    }
}

/// Arbitrary region/province map (Hearts of Iron style).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionMap {
    pub cells: HashMap<String, HashSet<String>>,
    pub cell_metadata: HashMap<String, Value>,
}

impl RegionMap {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            cell_metadata: HashMap::new(),
        }
    }
    pub fn add_cell(&mut self, id: &str) {
        self.cells.entry(id.to_string()).or_default();
    }
    pub fn add_neighbor(&mut self, from: &str, to: &str) {
        self.cells
            .entry(from.to_string())
            .or_default()
            .insert(to.to_string());
    }
}

impl Default for RegionMap {
    fn default() -> Self {
        Self::new()
    }
}

impl MapTopology for RegionMap {
    fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        if let CellKey::Region { id } = cell {
            self.cells
                .get(id)
                .map(|set| {
                    set.iter()
                        .map(|nid| CellKey::Region { id: nid.clone() })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            vec![]
        }
    }
    fn contains(&self, cell: &CellKey) -> bool {
        matches!(cell, CellKey::Region { id } if self.cells.contains_key(id))
    }
    fn all_cells(&self) -> Vec<CellKey> {
        self.cells
            .keys()
            .map(|id| CellKey::Region { id: id.clone() })
            .collect()
    }
    fn topology_type(&self) -> &'static str {
        "region"
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        if let CellKey::Region { id } = cell {
            self.cell_metadata.insert(id.clone(), data);
        }
    }
    fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        if let CellKey::Region { id } = cell {
            self.cell_metadata.get(id)
        } else {
            None
        }
    }
}

/// The main Map type (boxed trait object for dynamic dispatch).
pub struct Map {
    pub topology: Box<dyn MapTopology>,
}

impl Map {
    pub fn new(topology: Box<dyn MapTopology>) -> Self {
        Self { topology }
    }

    pub fn from_json(value: &Value) -> Option<Self> {
        let topology = value.get("topology")?.as_str()?;
        match topology {
            "square" => {
                let mut map = SquareGridMap::new();
                for cell in value.get("cells")?.as_array()? {
                    let x = cell.get("x")?.as_i64()? as i32;
                    let y = cell.get("y")?.as_i64()? as i32;
                    let z = cell.get("z")?.as_i64()? as i32;
                    map.add_cell(x, y, z);
                    if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                        for n in neighs {
                            let nx = n.get("x")?.as_i64()? as i32;
                            let ny = n.get("y")?.as_i64()? as i32;
                            let nz = n.get("z")?.as_i64()? as i32;
                            map.add_neighbor((x, y, z), (nx, ny, nz));
                        }
                    }
                    // Load cell metadata if present
                    if let Some(meta) = cell.get("metadata") {
                        let key = CellKey::Square { x, y, z };
                        map.set_cell_metadata(&key, meta.clone());
                    }
                }
                Some(Map::new(Box::new(map)))
            }
            "hex" => {
                let mut map = HexGridMap::new();
                for cell in value.get("cells")?.as_array()? {
                    let q = cell.get("q")?.as_i64()? as i32;
                    let r = cell.get("r")?.as_i64()? as i32;
                    let z = cell.get("z")?.as_i64()? as i32;
                    map.add_cell(q, r, z);
                    if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                        for n in neighs {
                            let nq = n.get("q")?.as_i64()? as i32;
                            let nr = n.get("r")?.as_i64()? as i32;
                            let nz = n.get("z")?.as_i64()? as i32;
                            map.add_neighbor((q, r, z), (nq, nr, nz));
                        }
                    }
                    if let Some(meta) = cell.get("metadata") {
                        let key = CellKey::Hex { q, r, z };
                        map.set_cell_metadata(&key, meta.clone());
                    }
                }
                Some(Map::new(Box::new(map)))
            }
            "region" => {
                let mut map = RegionMap::new();
                for cell in value.get("cells")?.as_array()? {
                    let id = cell.get("id")?.as_str()?.to_string();
                    map.add_cell(&id);
                    if let Some(neighs) = cell.get("neighbors").and_then(|n| n.as_array()) {
                        for n in neighs {
                            let nid = n.as_str()?.to_string();
                            map.add_neighbor(&id, &nid);
                        }
                    }
                    if let Some(meta) = cell.get("metadata") {
                        let key = CellKey::Region { id: id.clone() };
                        map.set_cell_metadata(&key, meta.clone());
                    }
                }
                Some(Map::new(Box::new(map)))
            }
            _ => None,
        }
    }

    pub fn contains(&self, cell: &CellKey) -> bool {
        self.topology.contains(cell)
    }
    pub fn neighbors(&self, cell: &CellKey) -> Vec<CellKey> {
        self.topology.neighbors(cell)
    }
    pub fn topology_type(&self) -> &'static str {
        self.topology.topology_type()
    }
    pub fn all_cells(&self) -> Vec<CellKey> {
        self.topology.all_cells()
    }

    pub fn set_cell_metadata(&mut self, cell: &CellKey, data: Value) {
        self.topology.set_cell_metadata(cell, data);
    }
    pub fn get_cell_metadata(&self, cell: &CellKey) -> Option<&Value> {
        self.topology.get_cell_metadata(cell)
    }
}
