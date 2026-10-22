# Disposition: vb-fyktj (fuzz-bound)

**Candidate:** 62a2f47c2f2874d39dcb61466698e3f737e7d316
**Main:** dba19b95918aca7c23e91b8814afa2a9118449f8
**Merge Base:** da633fe0bbde9e414173d5f7aabe314a37f8752d

## Summary

Disposition: **PARTIALLY_USEFUL** — 2 of 3 changes not yet landed; 1 change already landed.

| File | Change | Status |
|------|--------|--------|
| `fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs` | Add doc comment with run command | NOT LANDED — useful |
| `fuzz/fuzz_targets/yaml_events.rs` | Add corpus seed doc comment | ALREADY LANDED — superseded |
| `fuzz/src/yaml_target.rs` | `MAX_FUZZ_PAYLOAD_USIZE` 4096 → 100000 | NOT LANDED — useful |

## Detailed Analysis

### 1. vb_f04l_yaml_compiler_compile.rs — NOT LANDED

**Candidate adds:**
```rust
//! Fuzz target for YAML compiler with strict profile validation.
//!
//! Command: cargo fuzz run vb_f04l_yaml_compiler_compile -- -max_len=65536 -runs=100000
```

**Current main (lines 1-5):**
```rust
#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::{CompileError, YamlCompiler};
```

No doc comment exists on main. This addition provides useful operational documentation for running the fuzz target.

### 2. yaml_events.rs — ALREADY LANDED

**Candidate adds:**
```rust
//!
//! Corpus seeds are maintained in `fuzz/corpus/yaml_events/`.
```

**Current main (lines 1-7):**
```rust
//! Fuzz target for YAML event parsing.
//!
//! This target verifies that YAML profile validation and event parsing never panic
//! on any UTF-8 input.
//!
//! Corpus seeds are maintained in `fuzz/corpus/yaml_events/`.
```

The corpus seed documentation is already present on main. This change is fully superseded.

### 3. yaml_target.rs — NOT LANDED

**Candidate changes:**
```rust
// Before: const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;
// After:  const MAX_FUZZ_PAYLOAD_USIZE: usize = 100000;
```

**Current main (line 16):**
```rust
const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;
```

Still at 4096. Production `YamlLimits::default()` in `crates/vb_yaml/src/yaml_limits.rs` defines:
- `max_nodes: 100_000`
- `max_source_bytes: 1_048_576`
- `max_sequence_len: 10_000`
- `max_scalar_bytes: 65_536`

The fuzz bound `MAX_FUZZ_PAYLOAD_USIZE` is used in assertion checks within `fuzz_yaml_events()` to validate event count and source map entry count do not exceed expectations. The current value of 4096 is well below the production `max_nodes` limit of 100,000, making it overly restrictive and potentially causing false assertion failures for legitimate large inputs. Raising to 100,000 aligns the fuzz sanity check with the actual production node limit.

## Semantic Bound Analysis

The `MAX_FUZZ_PAYLOAD_USIZE` constant is not a YAML parsing limit but a fuzz-target internal sanity assertion bound. It checks that the number of parsed events or source map entries does not exceed an expected maximum during fuzzing. This is distinct from the production `YamlLimits` struct which is enforced during actual compilation.

The candidate's choice of 100,000 is well-reasoned because:
1. It matches the production `max_nodes` default, providing a consistent reference point
2. It is significantly larger than the current 4096, reducing risk of false positives
3. It remains well below `max_source_bytes` (1 MB), so a payload could still be "too large" for other reasons

## Confidence

**HIGH** — All three files were read in full from the current main tree. The patch was applied conceptually and compared against actual main content. Production limits were verified directly from `crates/vb_yaml/src/yaml_limits.rs`.

## Gaps

- No execution of the fuzz targets was performed (per instructions). The semantic soundness of the bound change relies on code inspection only.
- The candidate patch does not include any tests for the new bound, which is acceptable for a constant change in a fuzz target.

## Recommendation

Merge the useful deltas:
1. Add doc comment to `vb_f04l_yaml_compiler_compile.rs`
2. Change `MAX_FUZZ_PAYLOAD_USIZE` to 100000 in `yaml_target.rs`

Skip the already-landed `yaml_events.rs` doc comment.
