# vb-fyktj Fuzz Bound Review v2 (Corrected)

## Factual Correction from v1

v1 report claimed the candidate patch "ADDS a Command max_len line" to `yaml_events.rs`. **This is incorrect.** The patch adds a **documentation comment** to `vb_f04l_yaml_compiler_compile.rs` and `yaml_events.rs`, not a runtime Command or bound enforcement change.

## Patch Analysis

**Candidate:** `62a2f47c2f2874d39dcb61466698e3f737e7d316`
**Main:** `dba19b95918aca7c23e91b8814afa2a9118449f8`

### Actual Changes
1. `fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs`: Adds doc comment with suggested fuzz command `-max_len=65536 -runs=100000`
2. `fuzz/fuzz_targets/yaml_events.rs`: Adds doc comment with suggested fuzz command `-max_len=1024 -runs=100000`

No source code changes. No bound enforcement changes. No behavioral changes.

## Bound Soundness Analysis

### Current Enforcement Architecture
- `YamlLimits::default()` defines `max_nodes: 100_000` (in `crates/vb_yaml/src/yaml_limits.rs`)
- `validate_yaml_profile()` calls `collect_and_validate_events()` which tracks depth, node counts, sequence/mapping entry counts
- **However:** `record_node()` method exists but is **never called** in the event processing loop (`apply_event`)
- Actual enforcement uses per-container limits: `max_depth=64`, `max_mapping_entries=1024`, `max_sequence_len=10000`
- `node_count` field and `record_node()` are defined but not integrated into the enforcement path

### Is 100,000 Events a Sound Bound?
**Not for all accepted YAML.** With the current per-container limits:
- Top-level mapping can have 1,024 entries
- Each value can be a sequence with 10,000 items
- Each item can be a mapping with 1,024 entries
- Total theoretical events: millions (limited by input size, not event count)

The fuzz target `fuzz_yaml_events` asserts `events.len() <= 4096` (MAX_FUZZ_PAYLOAD_USIZE), which is a **fuzz-specific assertion**, not a parser bound. This is reasonable for fuzzing but not a general soundness bound.

### The `-runs=100000` in the Patch
This is documentation of a suggested fuzzing iteration count, not a bound on events or nodes. It has no effect on parser behavior or soundness.

## Verdict

| Claim | Status | Evidence |
|-------|--------|----------|
| Patch adds Command max_len line | **FALSE** | Patch adds doc comment only |
| Patch changes bound enforcement | **FALSE** | No source code changes |
| 100,000 event bound is sound | **INCONCLUSIVE** | `max_nodes=100000` defined but not enforced; per-container limits allow millions of events |
| `-runs=100000` is sound | **TRUE (trivial)** | Documentation only, no runtime effect |
| Patch is safe to land | **HIGH** | Pure documentation change, no behavioral impact |

## Recommendations

1. **If bound enforcement is desired:** Integrate `record_node()` into `apply_event()` so `max_nodes=100000` actually limits total nodes
2. **If fuzz assertion is too strict:** Consider increasing `MAX_FUZZ_PAYLOAD_USIZE` from 4096 to accommodate valid documents with many events
3. **Current patch:** Safe to land as documentation; does not establish or enforce any bound

## Risk

**LOW.** Pure documentation change. No behavioral impact on fuzzing or parser. The only risk is if readers interpret the doc comment as establishing a formal bound, which it does not.
