import os

import mge


def test_spawn_and_set_component():
    # Compute absolute path to schemas for robustness
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    world = mge.PyWorld(schema_dir)
    eid = world.spawn()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    comp = world.get_component(eid, "Health")
    print("Component:", comp)
