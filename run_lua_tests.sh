#!/usr/bin/env bash

set -e

cargo run --quiet --bin mge_lua_test_runner -- "$@"
