import pytest


def test_unknown_pair_defaults(make_world):
    world = make_world()
    assert world.get_relation("athens", "sparta") == "neutral"
    assert world.get_standing("athens", "sparta") == 0


def test_modify_standing_roundtrip(make_world):
    world = make_world()
    world.modify_standing("athens", "sparta", 30)
    assert world.get_standing("athens", "sparta") == 30
    assert world.get_standing("sparta", "athens") == 30


def test_standing_clamps_at_bounds(make_world):
    world = make_world()
    world.modify_standing("athens", "sparta", 200)
    assert world.get_standing("athens", "sparta") == 100
    world.modify_standing("athens", "sparta", -350)
    assert world.get_standing("athens", "sparta") == -100


def test_alliance_accept_sets_allied(make_world):
    world = make_world()
    tid = world.propose_treaty("athens", "sparta", "alliance")
    world.accept_treaty(tid)
    assert world.get_relation("athens", "sparta") == "allied"


def test_war_then_peace_resets_pair(make_world):
    world = make_world()
    tid = world.propose_treaty("athens", "sparta", "alliance")
    world.accept_treaty(tid)
    world.declare_war("athens", "sparta")
    assert world.get_relation("athens", "sparta") == "war"
    world.declare_peace("athens", "sparta")
    assert world.get_relation("athens", "sparta") == "neutral"
    assert world.get_standing("athens", "sparta") == 0


def test_peace_treaty_accept_ends_war(make_world):
    world = make_world()
    world.declare_war("athens", "corinth")
    tid = world.propose_treaty("athens", "corinth", "peace")
    world.accept_treaty(tid)
    assert world.get_relation("athens", "corinth") == "neutral"
    assert world.get_standing("athens", "corinth") == 0


def test_war_blocks_non_peace_proposal(make_world):
    world = make_world()
    world.declare_war("athens", "sparta")
    with pytest.raises(ValueError, match="at war"):
        world.propose_treaty("athens", "sparta", "non_aggression")


def test_self_pair_proposal_errors(make_world):
    world = make_world()
    with pytest.raises(ValueError, match="self"):
        world.propose_treaty("athens", "athens", "alliance")


def test_duplicate_proposal_errors(make_world):
    world = make_world()
    world.propose_treaty("athens", "sparta", "alliance")
    with pytest.raises(ValueError, match="duplicate"):
        world.propose_treaty("athens", "sparta", "alliance")


def test_break_applies_penalty(make_world):
    world = make_world()
    tid = world.propose_treaty("athens", "sparta", "alliance")
    world.accept_treaty(tid)
    world.break_treaty(tid)
    assert world.get_standing("athens", "sparta") == -25


def test_list_treaties_reports_records(make_world):
    world = make_world()
    world.accept_treaty(world.propose_treaty("athens", "sparta", "alliance"))
    world.propose_treaty("athens", "corinth", "trade")
    all_treaties = world.list_treaties()
    assert len(all_treaties) == 2
    assert all_treaties[0]["kind"] == "alliance"
    assert all_treaties[0]["status"] == "active"
    assert all_treaties[1]["kind"] == "trade"
    filtered = world.list_treaties("corinth")
    assert len(filtered) == 1
    assert filtered[0]["kind"] == "trade"
