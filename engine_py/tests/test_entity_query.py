import os

import mge


def test_get_entities_with_component():
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    world = mge.PyWorld(schema_dir)

    eid1 = world.spawn()
    eid2 = world.spawn()

    world.set_component(eid1, "Health", {"current": 5, "max": 10})

    ids = world.get_entities_with_component("Health")

    assert eid1 in ids
    assert eid2 not in ids


def test_get_entities():
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    world = mge.PyWorld(schema_dir)

    eid1 = world.spawn()
    eid2 = world.spawn()

    all_ids = world.get_entities()

    assert eid1 in all_ids
    assert eid2 in all_ids
