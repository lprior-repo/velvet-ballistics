#!/bin/bash
set -e
cd /home/lewis/src/isoloated/vb-ct9lz-impl-local5090-v4
cargo test --no-run --package vb_storage --lib --all-features --locked 2>&1 | tee .beads/vb-ct9lz/test-repair-v6/COMPILE.stderr