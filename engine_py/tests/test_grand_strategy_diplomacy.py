"""Grand-strategy diplomacy scenario: three rival nations (valoria, drakmor,
kesh) driven through the public diplomacy API only — initial relations,
goodwill standing, alliance treaty, war on the third nation, peace, and
treaty-list lifecycle."""


def test_three_nations_start_neutral(make_world):
    world = make_world()
    assert world.get_relation("valoria", "drakmor") == "neutral"
    assert world.get_relation("valoria", "kesh") == "neutral"
    assert world.get_relation("drakmor", "kesh") == "neutral"
    assert world.get_standing("valoria", "drakmor") == 0
    assert world.get_standing("valoria", "kesh") == 0
    assert world.get_standing("drakmor", "kesh") == 0


def test_goodwill_gesture_lifts_one_pair(make_world):
    world = make_world()
    world.modify_standing("valoria", "drakmor", 40)
    assert world.get_standing("valoria", "drakmor") == 40
    assert world.get_standing("drakmor", "valoria") == 40
    assert world.get_standing("valoria", "kesh") == 0


def test_alliance_unites_two_nations(make_world):
    world = make_world()
    tid = world.propose_treaty("valoria", "drakmor", "alliance")
    world.accept_treaty(tid)
    assert world.get_relation("valoria", "drakmor") == "allied"
    assert world.get_relation("valoria", "kesh") == "neutral"


def test_war_on_third_leaves_alliance_intact(make_world):
    world = make_world()
    tid = world.propose_treaty("valoria", "drakmor", "alliance")
    world.accept_treaty(tid)
    world.declare_war("valoria", "kesh")
    assert world.get_relation("valoria", "kesh") == "war"
    assert world.get_relation("kesh", "valoria") == "war"
    assert world.get_relation("valoria", "drakmor") == "allied"


def test_war_breaks_alliance_then_peace_ends_war(make_world):
    world = make_world()
    tid = world.propose_treaty("valoria", "drakmor", "alliance")
    world.accept_treaty(tid)
    world.declare_war("valoria", "drakmor")
    assert world.get_relation("valoria", "drakmor") == "war"
    world.declare_peace("valoria", "drakmor")
    assert world.get_relation("valoria", "drakmor") == "neutral"
    assert world.get_standing("valoria", "drakmor") == 0


def test_treaty_list_reflects_lifecycle(make_world):
    world = make_world()
    alliance = world.propose_treaty("valoria", "drakmor", "alliance")
    world.accept_treaty(alliance)
    world.declare_war("valoria", "kesh")
    world.declare_peace("valoria", "kesh")
    assert world.get_relation("valoria", "kesh") == "neutral"
    all_treaties = world.list_treaties()
    assert len(all_treaties) == 1
    assert all_treaties[0]["kind"] == "alliance"
    assert all_treaties[0]["status"] == "active"
    assert world.list_treaties("kesh") == []
    valoria_only = world.list_treaties("valoria")
    assert len(valoria_only) == 1
