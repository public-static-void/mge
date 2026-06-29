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


def test_reputation_decay(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_faction(eid, "goblins", "member")
    world.set_component(eid, "Reputation", {"values": {"goblins": 50}, "decay_rate": 1.0})
    for _ in range(3):
        world.tick()
    rep = world.get_reputation(eid, "goblins")
    # 50 decays by 1 per tick for 3 ticks = 47, stays above 0
    assert rep == 47


def test_reputation_decay_to_zero(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_faction(eid, "goblins", "member")
    world.set_component(eid, "Reputation", {"values": {"goblins": 3}, "decay_rate": 1.0})
    for _ in range(5):
        world.tick()
    rep = world.get_reputation(eid, "goblins")
    # 3 decays by 1 per tick to 0, then stops (does not cross zero)
    assert rep == 0


def test_reputation_no_decay_zero_rate(make_world):
    world = make_world()
    eid = world.spawn_entity()
    world.set_faction(eid, "goblins", "member")
    world.set_component(eid, "Reputation", {"values": {"goblins": 50}, "decay_rate": 0.0})
    for _ in range(3):
        world.tick()
    rep = world.get_reputation(eid, "goblins")
    # No decay when rate is 0.0
    assert rep == 50
