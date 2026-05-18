bead_id: vb-v7x6
bead_title: quality: repair doc gate ui release test
phase: 1
updated_at: 2026-05-18T00:00:00Z
attempt: 1-of-7

# State

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /tmp/opencode/go-skill-vb-v7x6
current_state: State 1 - Isolation and baseline

# Path Guard Evidence

- `pwd -P` in isolated workspace: `/tmp/opencode/go-skill-vb-v7x6`
- Guard: isolated workspace is not equal to source checkout and is not nested under source checkout.
- Source checkout remains control-plane only.

# Retry Counters

- State 1: attempt 1/7

# Routing

Next: capture baseline-report.md, then State 2 scope exact doc gate failure.
