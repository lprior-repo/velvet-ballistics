# Fuzz Bound Disposition - vb-fyktj

Candidate: 62a2f47c2 vs main dba19b95
Reviewed: 2026-10-22

## Deltas

Three actual deltas in the candidate patch:

1. `fuzz/fuzz_targets/yaml_events.rs`: Added doc comment with recommended fuzz command (`-max_len=65536 -runs=100000`). Documentation only, no behavioral change.

2. `fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs`: Added doc comment with recommended fuzz command (`-max_len=65536 -runs=100000`). Documentation only, no behavioral change.

3. `fuzz/src/yaml_target.rs`: Changed `MAX_FUZZ_PAYLOAD_USIZE` from `4096` to `100_000`. Behavioral change affecting fuzz target assertions.

## Soundness Analysis

The `MAX_FUZZ_PAYLOAD_USIZE` constant is used in `fuzz_yaml_events` to assert:
- `events.len() <= MAX_FUZZ_PAYLOAD_USIZE` after `parse_yaml_events` succeeds
- `source_map.len() <= MAX_FUZZ_PAYLOAD_USIZE` after `build_source_map` succeeds

### Production enforcement chain

`parse_yaml_events(text)` calls:
1. `yaml_profile::validate_yaml_profile(text)` → `collect_and_validate_events(text, &YamlLimits::default())`
   - `YamlLimits::default().max_nodes = 100_000` (crates/vb_yaml/src/yaml_limits.rs:33)
   - `collect_and_validate_events` calls `record_node(limits)` for every event (crates/vb_yaml/src/event_collect.rs:22)
   - `record_node` increments `node_count` and returns `YamlError::NodeLimitExceeded` if `node_count > limits.max_nodes` (crates/vb_yaml/src/event_collect.rs:193-207)
   - If any event exceeds the limit, validation fails and `parse_yaml_events` returns `Err`
2. `yaml_events::collect_events(text)` — second collection pass, no additional limit check

When `parse_yaml_events` returns `Ok(events)`, the validation step has already confirmed `events.len() <= 100_000`.

`build_source_map(text)` has the same validation-first pattern:
1. `validate_yaml_profile(text)` — enforces max_nodes=100,000
2. `collect_events(text)` — collects events
3. `source_map_from_events(text, &events)` — creates source map entries only for `MappingStart`, `SequenceStart`, `Scalar` events (crates/vb_yaml/src/yaml_source_map_build.rs:39-50)

Source map entries are a subset of events, so `source_map.len() <= events.len() <= 100_000`.

### Bound correctness

The old bound of 4096 was unsound: valid YAML with up to 100,000 events would pass production validation but fail the fuzz assertion, producing false positives.

The new bound of 100_000 matches the production `YamlLimits::default().max_nodes` exactly. This is the correct, tightest possible bound. The assertion fires only if production validation has a bug (events collected beyond the node limit).

## Disposition

- Doc comments (deltas 1-2): Useful, accurate, no risk.
- Bound change (delta 3): Semantically sound, matches production enforcement. Required for correctness — the previous bound of 4096 was too small and would cause false fuzz failures on valid workflows.
- Overall: Approved. The bound change is not documentation-only; it is a behavioral fix that aligns the fuzz target with production limits.
