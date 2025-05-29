def test_camera_api(make_world):
    world = make_world()
    world.set_camera(3, 7)
    cam = world.get_camera()
    assert isinstance(cam, dict)
    assert cam["x"] == 3
    assert cam["y"] == 7

    world.set_camera(10, 2)
    cam2 = world.get_camera()
    assert cam2["x"] == 10
    assert cam2["y"] == 2
