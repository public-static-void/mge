def test_get_job_type_metadata(make_world):
    world = make_world()
    job_types = world.get_job_types()
    assert isinstance(job_types, list)
    assert "DigTunnel" in job_types

    meta = world.get_job_type_metadata("DigTunnel")
    assert isinstance(meta, dict)
    assert meta["name"] == "DigTunnel"
    assert "effects" in meta
    assert isinstance(meta["effects"], list)
    assert "requirements" in meta
    assert meta["requirements"] == ["Tool:Pickaxe"]
    assert meta["duration"] == 5
    assert meta["effects"][0]["action"] == "ModifyTerrain"
    assert meta["effects"][0]["from"] == "rock"
    assert meta["effects"][0]["to"] == "tunnel"
