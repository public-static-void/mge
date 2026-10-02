//! Standing suite for procedural history and lore generation.
//!
//! Created in M1 alongside the core lore types and World persistence
//! wiring. Grows milestone by milestone (M2 extends in place with feed and
//! query coverage; M3 with rendering and backfill coverage).

#[path = "helpers/world.rs"]
mod world_helper;
#[path = "helpers/world_io.rs"]
mod world_io_helper;

use world_helper::make_test_world;
use world_io_helper::save_and_load_roundtrip;

use engine_core::lore::{ChronicleKind, append_chronicle_entry, chronicle_len};

/// Saves a world holding mixed-kind chronicle entries and asserts the load
/// preserves entries, RNG state, and the id counter, with ids continuing
/// monotonically on the next append.
#[test]
fn roundtrip_preserves_entries_rng_state_and_id_sequence() {
    let mut world = make_test_world();
    world.lore.rng_state = [5u8; 32];

    let first = append_chronicle_entry(
        &mut world,
        2,
        "food_shortage",
        ChronicleKind::Fired,
        "Turn 2: Food Shortage -- fired".to_string(),
        None,
    );
    let second = append_chronicle_entry(
        &mut world,
        2,
        "food_shortage",
        ChronicleKind::Resolved,
        "Turn 2: Food Shortage -- resolved [ration]".to_string(),
        Some("ration".to_string()),
    );
    let third = append_chronicle_entry(
        &mut world,
        4,
        "founding:first_hearth",
        ChronicleKind::Founding,
        "Turn 4: Founding of the First Hearth -- founding".to_string(),
        None,
    );
    assert_eq!((first, second, third), (0, 1, 2));
    assert_eq!(chronicle_len(&world), 3);

    let registry = world.registry.clone();
    let mut loaded = save_and_load_roundtrip(&world, registry);

    assert_eq!(loaded.lore.entries, world.lore.entries);
    assert_eq!(loaded.lore.rng_state, world.lore.rng_state);
    assert_eq!(loaded.lore.next_entry_id, 3);

    let next = append_chronicle_entry(
        &mut loaded,
        6,
        "food_shortage",
        ChronicleKind::Expired,
        "Turn 6: Food Shortage -- expired".to_string(),
        None,
    );
    assert_eq!(next, 3, "ids continue monotonically after load");
    assert_eq!(chronicle_len(&loaded), 4);
    assert_eq!(loaded.lore.next_entry_id, 4);
}

/// A save written before lore existed loads with an empty chronicle.
#[test]
fn save_without_lore_fields_loads_with_empty_chronicle() {
    let world = make_test_world();

    let mut json: serde_json::Value = serde_json::to_value(&world).expect("world serializes");
    json.as_object_mut()
        .expect("world is a JSON object")
        .remove("lore");

    let loaded: engine_core::ecs::world::World =
        serde_json::from_value(json).expect("old save loads");
    assert_eq!(chronicle_len(&loaded), 0);
    assert!(loaded.lore.entries.is_empty());
    assert_eq!(loaded.lore.next_entry_id, 0);
}
