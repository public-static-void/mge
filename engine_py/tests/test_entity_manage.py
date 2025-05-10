import os

import mge


def make_world():
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    return mge.PyWorld(schema_dir)


def test_despawn_and_remove_component():
    world = make_world()
    eid = world.spawn()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    world.remove_component(eid, "Health")
    assert world.get_component(eid, "Health") is None
    world.despawn(eid)
    assert eid not in world.get_entities()


def test_is_entity_alive():
    world = make_world()
    eid = world.spawn()
    world.set_component(eid, "Health", {"current": 10, "max": 10})
    assert world.is_entity_alive(eid)
    world.set_component(eid, "Health", {"current": 0, "max": 10})
    assert not world.is_entity_alive(eid)
