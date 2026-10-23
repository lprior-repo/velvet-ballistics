# Test Review: vb-h2qc2 yaml fuzz bound regression

Bead: vb-h2qc2
Candidate: 4df9e4f196e3047a280cb2d1755488afd1bfd60b (4096 -> 100000 bound fix)
Review version: test-review-v7
Reviewer: independent test reviewer

## Scope

One false-positive regression in `fuzz/src/yaml_target.rs`: `MAX_FUZZ_PAYLOAD_USIZE` set to 4096, too small for valid public-profile YAML that parses to >4096 events. Fix: constant to 100000 plus two doc max_len commands. Tests already on main; beads stale awaiting acceptance.

## Source Under Review

Single test: `fuzz/src/yaml_target.rs` module `tests::large_yaml_exceeds_4096_bound`.

## Verification Execution (actual logs reviewed)

- V2 RED: `cargo test -p velvet-ballistics-fuzz --lib yaml_target::tests::large_yaml_exceeds_4096_bound` -> FAILED with panic "event count 5010 exceeds max payload bound" at yaml_target.rs:65:17. Exit 101. 35.57s.
- V3 GREEN: same command after constant fix -> 1 passed. Exit 0. 2.99s.
- V4 ALL_LIBRARY: `cargo test -p velvet-ballistics-fuzz --lib` -> 3 passed, 0 failed. Exit 0. 2.97s.

Fresh canonical all-workspace including fuzz member (V82) in progress in integrator; exact scoped rerun after canonical pending. No pretending it ran.

## Test Behavior Analysis

Test constructs valid YAML: mapping `items` with sequence of 5001 scalars (`item0`...`item5000`).

Production limit compliance verified:
- Sequence length: 5001 < 10000 (max_sequence_len)
- Nesting depth: 2 (root + sequence) < 64 (max_depth)
- Source bytes: ~60KB < 1048576 (max_source_bytes)
- Node count: 5010 events < 100000 (max_nodes)

Test calls real production APIs:
- `vb_compile::validate_yaml_profile(&yaml)?` - validates public profile
- `vb_compile::parse_yaml_events(&yaml)?` - parses actual events
- `vb_compile::build_source_map(&yaml)?` - builds source map
- `fuzz_yaml_events(yaml.as_bytes())` - calls production fuzz target

Contract-shaped assertions:
- `events.len() > 4096` - proves test generates enough events to exceed old bound
- `events.len() <= 100000` - proves within default max_nodes
- `source_map.len() > 4096` - proves source map also exceeds old bound
- `source_map.len() <= 100000` - proves within default max_nodes

No catch/unwind/should_panic/ignore: test relies on `assert!` inside production `fuzz_yaml_events()`.

## Mutation Resistance

Restoring constant to 4096 causes V2 RED failure: panic "event count 5010 exceeds max payload bound". Test catches the exact regression.

## Test Review Gate Assessment

| Gate | Status |
|------|--------|
| Tests compile and execute deterministically | PASS (V2, V3, V4 all ran) |
| No ignored tests | PASS (0 ignored) |
| No sleeps/broad mocks/hidden state | PASS |
| Concrete assertions, not is_ok() smoke | PASS (exact >4096/<=100000 inequalities) |
| Named test catches restoring old bound | PASS (V2 RED proves it) |
| Tests same authored body red then green | PASS (V2=RED, V3=GREEN, identical test) |
| No universal all-YAML claim | PASS (scoped to one regression) |
| No performance/fuzz-campaign/formal proof claim | PASS |
| Bounded execution | PASS (finite 5001 loop, default limits) |

## Findings

**Finding 1 (OBSERVATION):** V4 log shows `cargo test -p velvet-ballistics-fuzz --lib` (entire fuzz library), not a fresh canonical all-workspace run. Fresh canonical all-workspace including fuzz member (V82) is in progress in integrator per manifest. This is expected for a test review focused on the bound regression; no action required for this bead. Disposition: owner_approved_debt.

No findings at any severity that are open. All verification gates pass.

## Conclusion

The test `large_yaml_exceeds_4096_bound` is a valid, production-bound regression test for the 4096->100000 fuzz assertion bound fix. V2 RED proves the old bound was too small for valid YAML (5010 events). V3 GREEN proves the fix. V4 ALL_LIBRARY proves no regression in the fuzz library. The test uses real valid YAML, calls the actual production fuzz target, and uses exact contract-shaped boolean inequality assertions.

STATUS: APPROVED
