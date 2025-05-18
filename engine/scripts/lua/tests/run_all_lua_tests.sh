#!/bin/sh
set -e
for f in test_*.lua; do
	echo "Running $f"
	lua "$f"
done
