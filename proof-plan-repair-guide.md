# Proof Plan Repair Guide: vb-wymp, vb-r5zb, vb-ui6k

## Review Result: REJECTED

Three blocker findings require repair before proof planning can proceed.

## REPAIR-001: Create proof planning artifacts for vb-wymp
- **Bead**: vb-wymp (storage: Extend DigestCheck::Full to verify action ABI and policy digests)
- **Current State**: No proof planning artifacts exist
- **Artifacts to Create**:
  1. `proof-seeds.jsonl` - Classify behavior_affecting (likely true since this modifies production Rust storage code)
  2. `verifier-lane-decisions.jsonl` - Default Rust lane profile: Verus, Kani, Flux, proptest
  3. `proof-obligations.planned.jsonl` - Machine-readable obligations with schema version, exact commands, workdir, bounds, assumptions, expected evidence
  4. `proof-strategy.md` - Summary of verification approach
  5. `traceability-matrix.jsonl` - Links seeds to obligations to source refs
  6. `trusted-base-plan.md` - Documents any trusted base claims
- **Minimal State**: 4 (proof-planner)
- **Justification**: This bead modifies `crates/vb_storage/src/recovery/recover.rs` which is production Rust code handling storage recovery. The verification-lane-policy requires default Rust lanes for any Rust behavior change.

## REPAIR-002: Clarify Miri verification scope for vb-r5zb
- **Bead**: vb-r5zb (ci: Expand Miri to run on all vb_core, vb_expr, vb_compile tests)
- **Current State**: No proof planning artifacts exist
- **Option A**: If Miri verification is claimed:
  - Create full proof planning artifacts with Miri lane decisions
  - Document what Miri is expected to catch in expanded test suites
- **Option B**: If purely CI tooling with no formal verification claims:
  - Document explicitly that no formal verification obligations exist
  - Close bead with "CI tooling only" classification
- **Minimal State**: 4 (proof-planner) or close without proof artifacts
- **Justification**: The bead expands Miri task in `.moon/tasks/all.yml`. If this expansion claims Miri verification value, it must be documented. If it's just CI infrastructure, formal proof is not required.

## REPAIR-003: Clarify vb-ui6k scope
- **Bead**: vb-ui6k (arch: Expand source-length gate to all first-party Rust files)
- **Current State**: No proof planning artifacts exist; bead was closed as "is_hot_source bug fix"
- **Option A**: If production Rust is affected (`scripts/source_length_scan.rs`):
  - Create full proof planning artifacts
  - This modifies hot-function detection logic
- **Option B**: If purely tooling/script (no production Rust behavior change):
  - Document explicitly that no formal verification obligations exist
  - Close bead with "tooling only" classification
- **Minimal State**: 4 (proof-planner) or close without proof artifacts
- **Justification**: The bead description mentions "is_hot_source bug fix" but the script `source_length_scan.rs` could be tooling only. Clarification is needed.

## Downstream Notes

- These beads were "driven to done" without proof planning artifacts
- The verification-lane-policy requires proof planning for any Rust behavior change
- vb-wymp clearly modifies production Rust and requires formal proof planning
- vb-r5zb and vb-ui6k may be CI/tooling changes that don't require formal verification

## Verification Lane Policy Reference

Default Rust-Implementation Profile (required for all Rust behavior):
- Verus for Rust-local pure/core invariants, arithmetic, indexing, typestate transitions
- Kani for bounded state, panic/overflow/index risk, error/rejection claims
- Flux for illegal states expressible as refinements (when practical)
- proptest for behavior/property pressure through executable Rust APIs

Conditional lanes:
- Loom for concurrency, cancellation, shutdown, task ownership risk
- Miri for unsafe, FFI, layout, aliasing, raw-pointer, UB-sensitive claims
- cargo-fuzz for parsers, codecs, hostile input boundaries

---

(End of file - total 74 lines)