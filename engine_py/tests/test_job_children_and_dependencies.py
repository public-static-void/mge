def test_job_children_and_dependencies(make_world):
    world = make_world()
    # Create parent job with no children or dependencies
    parent_id = world.spawn_entity()
    world.assign_job(parent_id, "ParentJob", state="pending", category="test")

    # Create two child jobs
    child1 = {
        "job_type": "ChildJob",
        "state": "pending",
        "category": "test",
        "progress": 0.0,
    }
    child2 = {
        "job_type": "ChildJob",
        "state": "pending",
        "category": "test",
        "progress": 0.0,
    }
    # Set children
    world.set_job_children(parent_id, [child1, child2])

    # Get children back and check structure
    children = world.get_job_children(parent_id)
    assert isinstance(children, list)
    assert len(children) == 2
    assert all(child["job_type"] == "ChildJob" for child in children)

    # Set dependencies (complex expr)
    deps = {
        "all_of": [
            "job:fetch_wood",
            {"any_of": ["job:mine_stone", "job:collect_clay"]},
            {"not": ["job:destroyed_bridge"]}
        ]
    }
    world.set_job_dependencies(parent_id, deps)

    # Get dependencies back and check structure
    got_deps = world.get_job_dependencies(parent_id)
    assert isinstance(got_deps, dict)
    assert "all_of" in got_deps
    assert got_deps["all_of"][0] == "job:fetch_wood"
    assert isinstance(got_deps["all_of"][1], dict)
    assert "any_of" in got_deps["all_of"][1]
    assert got_deps["all_of"][2]["not"][0] == "job:destroyed_bridge"

    # Overwrite children with empty list
    world.set_job_children(parent_id, [])
    children = world.get_job_children(parent_id)
    assert children == []

    # Overwrite dependencies with a simple string array
    world.set_job_dependencies(parent_id, ["job:foo", "job:bar"])
    got_deps = world.get_job_dependencies(parent_id)
    assert got_deps == ["job:foo", "job:bar"]
