import pytest


def test_dynamic_system_registration(make_world):
    world = make_world()
    ran = {"flag": False}

    def sys(dt):
        print("PYTHON SYSTEM CALLED")
        ran["flag"] = True

    world.register_system("test_py_system", sys)
    world.run_system("test_py_system")
    assert ran["flag"] is True
