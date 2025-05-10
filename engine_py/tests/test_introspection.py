import os

import mge


def make_world():
    here = os.path.dirname(__file__)
    schema_dir = os.path.abspath(
        os.path.join(here, "../../engine/assets/schemas")
    )
    return mge.PyWorld(schema_dir)


def test_list_components_and_schema():
    world = make_world()
    comps = world.list_components()
    assert "Health" in comps
    schema = world.get_component_schema("Health")
    assert isinstance(schema, dict)
    assert "properties" in schema
    assert "current" in schema["properties"]
