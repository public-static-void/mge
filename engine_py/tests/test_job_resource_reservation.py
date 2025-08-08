def test_job_resource_reservation_query(make_world):
    world = make_world()
    e = world.spawn_entity()
    world.set_component(
        e,
        "Job",
        {
            "job_type": "Build",
            "category": "construction",
            "state": "pending",
            "reserved_resources": [
                {"kind": "wood", "amount": 3},
                {"kind": "stone", "amount": 1},
            ],
        },
    )

    res = world.get_job_resource_reservations(e)
    print("Query test: reserved resources =", res)
    assert res is not None
    assert res[0]["kind"] == "wood"
    assert res[0]["amount"] == 3
    assert res[1]["kind"] == "stone"
    assert res[1]["amount"] == 1

    # Should return None if no reserved_resources field
    world.set_component(
        e,
        "Job",
        {"job_type": "Build", "category": "construction", "state": "pending"},
    )
    none = world.get_job_resource_reservations(e)
    print("Query test: reserved resources after removal =", none)
    assert none is None


def test_job_resource_reservation_mutation(make_world):
    world = make_world()
    stockpile = world.spawn_entity()
    world.set_component(stockpile, "Stockpile", {"resources": {"wood": 10}})
    print("Mutation test: stockpile entity =", stockpile)
    print(
        "Mutation test: stockpile resources before =",
        world.get_component(stockpile, "Stockpile"),
    )

    e = world.spawn_entity()
    world.set_component(
        e,
        "Job",
        {
            "job_type": "Build",
            "category": "construction",
            "state": "pending",
            "resource_requirements": [
                {"kind": "wood", "amount": 3},
            ],
        },
    )
    print("After job creation:", world.get_component(e, "Job"))

    assert world.get_job_resource_reservations(e) is None

    # Run the resource reservation system explicitly before reserving resources
    world.run_resource_reservation_system()

    reserved = world.reserve_job_resources(e)
    print("Mutation test: reserved result =", reserved)
    print(
        "Mutation test: job component after reserve =",
        world.get_component(e, "Job"),
    )

    res = world.get_job_resource_reservations(e)
    print("Mutation test: reserved resources after reserve =", res)
    assert reserved is True
    assert res is not None
    assert res[0]["kind"] == "wood"
    assert res[0]["amount"] == 3

    # Now release
    world.release_job_resource_reservations(e)
    print(
        "Mutation test: job component after release =",
        world.get_component(e, "Job"),
    )
    assert world.get_job_resource_reservations(e) is None
