def test_time_of_day_advances(make_world):
    world = make_world()
    tod = world.get_time_of_day()
    assert tod["hour"] == 0
    assert tod["minute"] == 0
    assert tod["day"] == 0
    assert tod["season"] == "spring"

    world.tick()
    tod = world.get_time_of_day()
    assert tod["minute"] == 1

    for _ in range(59):
        world.tick()
    tod = world.get_time_of_day()
    assert tod["hour"] == 1
    assert tod["minute"] == 0


def test_day_increments_after_full_day(make_world):
    world = make_world()
    for _ in range(24 * 60):
        world.tick()
    tod = world.get_time_of_day()
    assert tod["day"] == 1
    assert tod["hour"] == 0
    assert tod["minute"] == 0
    assert tod["season"] == "spring"


def test_season_changes_to_summer(make_world):
    world = make_world()
    # 30 days = spring, advance to day 30
    for _ in range(30 * 24 * 60):
        world.tick()
    tod = world.get_time_of_day()
    assert tod["day"] == 30
    assert tod["season"] == "summer"
