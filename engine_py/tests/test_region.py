def test_region_queries(make_world):
    world = make_world()
    eid1 = world.spawn_entity()
    world.set_component(eid1, "Region", {"id": "room_1", "kind": "room"})
    eid2 = world.spawn_entity()
    world.set_component(
        eid2, "Region", {"id": ["room_1", "biome_A"], "kind": "room"}
    )
    eid3 = world.spawn_entity()
    world.set_component(eid3, "Region", {"id": "biome_A", "kind": "biome"})

    e_room = world.get_entities_in_region("room_1")
    assert len(e_room) == 2, f"room_1 should have 2 entities, got {e_room}"
    e_biome = world.get_entities_in_region("biome_A")
    assert len(e_biome) == 2, f"biome_A should have 2 entities, got {e_biome}"

    e_kind_room = world.get_entities_in_region_kind("room")
    assert (
        len(e_kind_room) == 2
    ), f"kind=room should have 2 entities, got {e_kind_room}"
    e_kind_biome = world.get_entities_in_region_kind("biome")
    assert (
        len(e_kind_biome) == 1
    ), f"kind=biome should have 1 entity, got {e_kind_biome}"


if __name__ == "__main__":
    test_region_queries()
    print("test_region_queries passed")
