def test_set_faction(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_faction(eid, "goblins", "member")
    assert world.get_faction(eid) == "goblins"

def test_get_faction_none(make_world):
    world = make_world()
    eid = world.spawn_entity()
    assert world.get_faction(eid) is None

def test_modify_and_get_reputation(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.modify_reputation(eid, "goblins", 25)
    assert world.get_reputation(eid, "goblins") == 25

def test_reputation_clamping_upper(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.modify_reputation(eid, "goblins", 200)
    assert world.get_reputation(eid, "goblins") == 100

def test_reputation_clamping_lower(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.modify_reputation(eid, "goblins", -200)
    assert world.get_reputation(eid, "goblins") == -100

def test_get_reputation_no_component(make_world):
    world = make_world()
    eid = world.spawn_entity()
    assert world.get_reputation(eid, "goblins") == 0

def test_get_reputation_unknown_faction(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.modify_reputation(eid, "goblins", 25)
    assert world.get_reputation(eid, "humans") == 0

def test_reputation_cumulative(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.modify_reputation(eid, "goblins", 10)
    world.modify_reputation(eid, "goblins", 20)
    assert world.get_reputation(eid, "goblins") == 30
