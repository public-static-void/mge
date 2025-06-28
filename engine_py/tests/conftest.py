import os
import pytest
import engine_py


@pytest.fixture
def make_world():
    def _make_world():
        engine_py.py_init_job_event_logger()
        here = os.path.dirname(__file__)
        schema_dir = os.path.abspath(
            os.path.join(here, "../../engine/assets/schemas")
        )
        return engine_py.PyWorld(schema_dir)

    return _make_world
