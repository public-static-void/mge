def test_camera_api(make_world):
    world = make_world()
    world.set_camera(3, 7)
    cam = world.get_camera()
    assert isinstance(cam, dict)
    assert cam["x"] == 3
    assert cam["y"] == 7
    assert cam["z"] == 0  # z defaults to 0 when omitted

    world.set_camera(10, 2)
    cam2 = world.get_camera()
    assert cam2["x"] == 10
    assert cam2["y"] == 2
    assert cam2["z"] == 0

    # Explicit z round-trip
    world.set_camera(3, 7, 2)
    cam3 = world.get_camera()
    assert cam3["x"] == 3
    assert cam3["y"] == 7
    assert cam3["z"] == 2
