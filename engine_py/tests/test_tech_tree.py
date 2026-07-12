"""Tests for the tech tree and research system.

Each test receives a `make_world` fixture from conftest.py that creates
a PyWorld instance with all schemas loaded and systems registered.
"""


def test_get_tech_tree(make_world):
    """get_tech_tree() returns a list of tech nodes."""
    world = make_world()
    tree = world.get_tech_tree()
    assert isinstance(tree, list)
    assert len(tree) > 0
    ids = [n["id"] for n in tree]
    assert "bronze_working" in ids


def test_get_tech_node(make_world):
    """get_tech_node() returns a specific node by ID."""
    world = make_world()
    node = world.get_tech_node("bronze_working")
    assert node is not None
    assert node["name"] == "Bronze Working"


def test_get_tech_node_missing(make_world):
    """get_tech_node() returns None for nonexistent tech."""
    world = make_world()
    node = world.get_tech_node("nonexistent_tech")
    assert node is None


def test_research_tech(make_world):
    """research_tech() adds a tech to the research queue."""
    world = make_world()
    eid = world.spawn_entity()
    world.research_tech(eid, "bronze_working")
    queue = world.get_research_queue(eid)
    assert len(queue) == 1
    assert queue[0] == "bronze_working"


def test_get_research_queue_progress(make_world):
    """get_research_queue_progress() returns progress map."""
    world = make_world()
    eid = world.spawn_entity()
    world.research_tech(eid, "bronze_working")
    progress = world.get_research_queue_progress(eid)
    assert isinstance(progress, dict)
    assert progress.get("bronze_working") == 0


def test_get_completed_techs(make_world):
    """get_completed_techs() returns list of completed techs."""
    world = make_world()
    eid = world.spawn_entity()
    # Set TechProgress with a completed tech
    world.set_component(eid, "TechProgress", {
        "completed": {"bronze_working": 1},
        "queue": [],
        "queue_progress": {},
        "research_points": 0,
    })
    completed = world.get_completed_techs(eid)
    assert "bronze_working" in completed


def test_is_tech_completed(make_world):
    """is_tech_completed() returns correct boolean."""
    world = make_world()
    eid = world.spawn_entity()
    assert not world.is_tech_completed(eid, "bronze_working")
    world.set_component(eid, "TechProgress", {
        "completed": {"bronze_working": 1},
        "queue": [],
        "queue_progress": {},
        "research_points": 0,
    })
    assert world.is_tech_completed(eid, "bronze_working")


def test_cancel_research(make_world):
    """cancel_research() removes tech from queue."""
    world = make_world()
    eid = world.spawn_entity()
    world.research_tech(eid, "bronze_working")
    assert len(world.get_research_queue(eid)) == 1
    world.cancel_research(eid, "bronze_working")
    assert len(world.get_research_queue(eid)) == 0


def test_clear_research_queue(make_world):
    """clear_research_queue() empties the queue."""
    world = make_world()
    eid = world.spawn_entity()
    world.research_tech(eid, "bronze_working")
    world.clear_research_queue(eid)
    assert len(world.get_research_queue(eid)) == 0


def test_can_research_tech(make_world):
    """can_research_tech() returns (bool, reason)."""
    world = make_world()
    eid = world.spawn_entity()
    ok, reason = world.can_research_tech(eid, "bronze_working")
    assert ok is True
    assert reason == ""


def test_cannot_research_completed_tech(make_world):
    """research_tech() raises on already completed tech."""
    world = make_world()
    eid = world.spawn_entity()
    world.set_component(eid, "TechProgress", {
        "completed": {"bronze_working": 1},
        "queue": [],
        "queue_progress": {},
        "research_points": 0,
    })
    try:
        world.research_tech(eid, "bronze_working")
        assert False, "Should have raised an error"
    except Exception as e:
        assert "already completed" in str(e)


def test_cannot_research_with_prereq(make_world):
    """research_tech() raises when prerequisite not met."""
    world = make_world()
    eid = world.spawn_entity()
    try:
        world.research_tech(eid, "iron_working")
        assert False, "Should have raised an error"
    except Exception as e:
        assert "Requires tech" in str(e)
