#!/usr/bin/env bash

set -e

cargo run --package engine_lua --bin mge_lua_test_runner --quiet -- "$@"
