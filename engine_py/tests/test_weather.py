# R017: get_weather() returns dict with expected keys
def test_get_weather_returns_dict(make_world):
    world = make_world()
    weather = world.get_weather()
    assert set(weather.keys()) == {"condition", "intensity", "duration_remaining"}
    assert weather["condition"] == "clear"
    assert weather["intensity"] == 0.0
    assert weather["duration_remaining"] == 0


# R017: set_weather() changes weather and get_weather() reflects it
def test_set_weather_changes_state(make_world):
    world = make_world()
    world.set_weather("rain", 0.8, 100)
    weather = world.get_weather()
    assert weather["condition"] == "rain"
    assert weather["intensity"] == 0.8
    assert weather["duration_remaining"] == 100


# R017: get_weather_visibility_modifier() returns correct value
def test_visibility_modifier_matches_formula(make_world):
    world = make_world()
    world.set_weather("rain", 0.8, 100)
    world.tick()
    vis = world.get_weather_visibility_modifier()
    # 0.7 * 0.8 = 0.56; compare with tolerance for f64 rounding
    assert abs(vis - 0.56) < 0.0001


def test_clear_has_full_visibility(make_world):
    world = make_world()
    world.set_weather("clear", 1.0, 100)
    world.tick()
    assert world.get_weather_visibility_modifier() == 1.0


# Unrecognized condition string maps to Clear per spec edge cases
def test_invalid_condition_defaults_to_clear(make_world):
    world = make_world()
    world.set_weather("thunder", 0.5, 100)
    weather = world.get_weather()
    assert weather["condition"] == "clear"


# Clamping: intensity above 1.0 is clamped
def test_intensity_clamped_to_1_0(make_world):
    world = make_world()
    world.set_weather("storm", 1.5, 100)
    weather = world.get_weather()
    assert weather["intensity"] == 1.0


# Clamping: negative intensity is clamped to 0.0
def test_intensity_clamped_to_0_0(make_world):
    world = make_world()
    world.set_weather("fog", -0.5, 100)
    weather = world.get_weather()
    assert weather["intensity"] == 0.0


# Duration 0 forces transition on next tick
def test_duration_zero_forces_transition(make_world):
    world = make_world()
    world.set_weather("cloudy", 0.5, 0)
    before = world.get_weather()
    assert before["duration_remaining"] == 0
    world.tick()
    after = world.get_weather()
    # After tick: duration expired, WeatherSystem transitions to new condition
    assert after["duration_remaining"] > 0