def test_time_of_day_advances(make_world):
    world = make_world()
    tod = world.get_time_of_day()
    assert tod["hour"] == 0
    assert tod["minute"] == 0

    world.tick()
    tod = world.get_time_of_day()
    assert tod["minute"] == 1

    for _ in range(59):
        world.tick()
    tod = world.get_time_of_day()
    assert tod["hour"] == 1
    assert tod["minute"] == 0
