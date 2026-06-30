---
section: 42
title: "Bead Work Breakdown"
parent: velvet-ballistics-MASTER.md
---

## 42. Bead Work Breakdown


Beads are the only task tracking mechanism for this repository. Every implementation phase must be decomposed into beads. Each bead must include:

```text
phase
crate/module
public API touched
resource contract impact
hot-path impact
storage impact
IPC impact
tests required
benchmarks required
acceptance commands
rollback/migration notes
```

Required bead groups:

```text
naming-migration
toolchain-ci
core-types
yaml-parser
validator
expression-engine
compiler-ir
runtime-engine
fjall-storage
recovery-replay
action-abi
shard-scheduler
ipc-protocol
cli
observability
tests-fuzz
benchmarks
release-gates
```

Every phase requires a parent bead. Every function cluster requires a child bead. The benchmark suite requires dedicated beads. Each fuzz target requires its own bead. Every P0 blocker requires a dedicated bead.

Dependency-scoping beads are mandatory when a library is added to reduce code footprint. They must record the removed handwritten code, semantic parity tests, hot-path allocation impact, and rollback decision. Current required dependency-scope beads:

```text
byteorder-ipc-little-endian-helpers
logos-expression-lexer-parity
indexmap-valuestore-object-field-index
ordered-float-finitef64-rejection-record
```

Round 2 black-hat findings remediated in this master document:

```text
hot-slotvalue-handle-only-model
finish-copy-out-compatible-with-copy-slotvalue
runframe-constructor-and-taint-api-contract
narrow-canonical-spelling-allowlist
hot-function-length-hard-gate
choose-ir-final-variant-disambiguation
action-abi-type-and-binary-semantics
persistence-record-envelope-byte-contract
mvp-wording-removed-from-final-ir-contract
```

Current black-hat/test-review gaps that are not optional phase polish require dedicated beads before final acceptance:

```text
compiler-full-v1-primitive-source-lowering
runtime-collect-next-pagination-state
runtime-admission-run-header-persistence
runtime-journal-sequence-hydration
runtime-full-live-frame-recovery-hydration
unsafe-fuzz-cabi-isolation
workspace-exact-assertion-sharpness
rust-test-loop-removal
silent-discard-elimination
test-plan-current-api-mutation-refresh
full-gate-evidence-refresh
```

Codegen and UI gaps are cleanup debt unless a bead explicitly deletes or quarantines residue. They are not reactivation tracks in the current master scope.

The previous `error-variant-completeness-audit` gap has Round 2 implementation evidence in `tests/error_variant_completeness_test.rs` and `docs/error-variant-completeness.md`; it remains subject to the full gate matrix like every other test surface.

Required first beads:

```text
naming
crate-package-folder-rebaseline
optional-language-removal
manual-ipc-triggers
slotvalue-handle-model
stepbudget-setconst-corrections
primitive-subphases
toolchain-nightly-governance
holzmann-matrix
forbidden-hot-path-apis
```

UI beads are limited to deletion or quarantine of residue unless the master scope is explicitly amended.

Example bead commands:

```bash
bd create --title="P0: canonical naming rebaseline" --description="Align product, binary, package, crate, bead rig, bead database, and language version spelling." --type=task --priority=0
bd create --title="Phase 13: deterministic engine MVP" --description="Implement SetConst/Copy/Choose/Finish with StepBudget and invariant tests." --type=feature --priority=0
bd dep add <child-bead> <phase-parent-bead>
bd update <bead-id> --claim
bd close <bead-id> --reason="Completed with tests, benchmarks, and CI evidence"
```

No phase is complete until all beads for that phase are closed with test/benchmark evidence and `bd dolt push` has synced the bead database.

---
