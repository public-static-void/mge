def test_set_and_get_base_stats(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {"strength": 10.0, "dexterity": 8.0, "intelligence": 6.0})
    stats = world.get_component(eid, "BaseStats")
    assert stats["strength"] == 10.0
    assert stats["dexterity"] == 8.0
    assert stats["intelligence"] == 6.0


def test_stat_pipeline_via_tick(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {"strength": 5.0, "constitution": 3.0})
    world.set_component(eid, "EquipmentEffects", {"strength": 3.0})
    world.tick()
    stats = world.get_component(eid, "Stats")
    assert stats is not None
    assert stats["strength"] == 8.0  # 5 + 3


def test_stat_pipeline_no_effects(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {"strength": 7.0, "constitution": 2.0})
    world.tick()
    stats = world.get_component(eid, "Stats")
    assert stats is not None
    assert stats["strength"] == 7.0


def test_derived_stats_via_tick(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {"strength": 10.0, "constitution": 5.0, "intelligence": 4.0})
    world.tick()
    derived = world.get_component(eid, "DerivedStats")
    assert derived is not None
    assert derived["MaxHP"] == 150.0  # 100 + 5*10
    assert derived["MeleeDamage"] == 6.0  # 1.0 + 10*0.5
    assert derived["CritChance"] == 0.07  # 0.05 + 4*0.005


def test_derived_stats_minimal(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {})
    world.tick()
    derived = world.get_component(eid, "DerivedStats")
    assert derived is not None
    assert derived["MaxHP"] == 100.0
    assert derived["MeleeDamage"] == 1.0
    assert derived["CritChance"] == 0.05


def test_skill_levels_component(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "SkillLevels", {
        "skills": {"mining": 3.0, "crafting": 2.0},
        "skill_levels": {"mining": 3.0, "crafting": 2.0},
        "total_xp": 60.0,
        "skill_xp": {"mining": 40.0, "crafting": 20.0}
    })
    levels = world.get_component(eid, "SkillLevels")
    assert levels is not None
    assert levels["skills"]["mining"] == 3.0
    assert levels["skills"]["crafting"] == 2.0
    assert levels["total_xp"] == 60.0


def test_query_entities_with_base_stats(make_world):
    world = make_world()
    eid1 = world.spawn_entity()
    eid2 = world.spawn_entity()
    eid3 = world.spawn_entity()
    world.set_component(eid1, "BaseStats", {"strength": 5.0})
    world.set_component(eid3, "BaseStats", {"strength": 8.0})
    entities = world.get_entities_with_component("BaseStats")
    assert len(entities) == 2


def test_equipment_effects_aggregation_via_tick(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {"strength": 5.0, "dexterity": 3.0})
    world.set_component(eid, "EquipmentEffects", {"strength": 2.0, "dexterity": 1.0})
    world.tick()
    stats = world.get_component(eid, "Stats")
    assert stats is not None
    assert stats["strength"] == 7.0
    assert stats["dexterity"] == 4.0


def test_derived_stats_update_on_re_tick(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "BaseStats", {"strength": 5.0, "constitution": 2.0})
    world.tick()
    derived = world.get_component(eid, "DerivedStats")
    assert derived["MeleeDamage"] == 1.0 + 5.0 * 0.5

    # Update BaseStats and re-tick
    world.set_component(eid, "BaseStats", {"strength": 20.0, "constitution": 2.0})
    world.tick()
    derived = world.get_component(eid, "DerivedStats")
    assert derived["MeleeDamage"] == 1.0 + 20.0 * 0.5
