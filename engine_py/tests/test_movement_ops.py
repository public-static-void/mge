def test_assign_move_path_and_queries(make_world):
    world = make_world()
    e_agent = world.spawn_entity()

    # Add map cells in a 3x3 grid covering (0,0,0) to (2,2,0)
    for x in range(3):
        for y in range(3):
            world.add_cell(x, y, 0)

    # Connect neighbors explicitly (required for pathfinding to find a route)
    for x in range(3):
        for y in range(3):
            if x < 2:
                world.add_neighbor((x, y, 0), (x + 1, y, 0))
                world.add_neighbor((x + 1, y, 0), (x, y, 0))
            if y < 2:
                world.add_neighbor((x, y, 0), (x, y + 1, 0))
                world.add_neighbor((x, y + 1, 0), (x, y, 0))

    # Set minimal Agent component on entity so assign_move_path can update it
    world.set_component(
        e_agent, "Agent", {"entity_id": e_agent, "move_path": []}
    )

    # Setup initial Position component for agent at (0, 0, 0)
    world.set_component(
        e_agent, "Position", {"pos": {"Square": {"x": 0, "y": 0, "z": 0}}}
    )

    from_cell = {"Square": {"x": 0, "y": 0, "z": 0}}
    to_cell = {"Square": {"x": 2, "y": 2, "z": 0}}

    # Assign move path to agent
    world.assign_move_path(e_agent, from_cell, to_cell)

    # Fetch Agent component and verify move_path exists and is not empty
    agent_comp = world.get_component(e_agent, "Agent")
    assert agent_comp is not None, "Agent component should exist"
    assert "move_path" in agent_comp, "Agent move_path should be set"
    assert len(agent_comp["move_path"]) > 0, "move_path should not be empty"

    # Check if agent is at from_cell (should be True)
    assert world.is_agent_at_cell(e_agent, from_cell) is True

    # Check if agent is at to_cell (should be False)
    assert world.is_agent_at_cell(e_agent, to_cell) is False

    # Check if move_path is not empty
    assert world.is_move_path_empty(e_agent) is False


def test_is_move_path_empty_with_no_agent_component(make_world):
    world = make_world()
    e_agent = world.spawn_entity()

    # No Agent component means move_path should be considered empty
    assert world.is_move_path_empty(e_agent) is True


def test_is_move_path_empty_when_absent_or_explicitly_empty(make_world):
    world = make_world()
    e_agent = world.spawn_entity()

    # Set Agent component but no move_path field - include required entity_id to satisfy schema
    world.set_component(e_agent, "Agent", {"entity_id": e_agent})
    assert world.is_move_path_empty(e_agent) is True

    # Set Agent component with empty move_path explicitly
    world.set_component(
        e_agent, "Agent", {"entity_id": e_agent, "move_path": []}
    )
    assert world.is_move_path_empty(e_agent) is True
