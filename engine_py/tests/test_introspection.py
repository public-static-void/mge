def test_list_components_and_schema(make_world):
    world = make_world()
    comps = world.list_components()
    assert "Health" in comps
    schema = world.get_component_schema("Health")
    assert isinstance(schema, dict)
    assert "properties" in schema
    assert "current" in schema["properties"]
