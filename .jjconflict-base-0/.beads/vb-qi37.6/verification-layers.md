# Verification Layers

## Boundary

- Verus-owned kernel: exact capability grant semantics, no-prefix grants, action equality, cardinality exactness, and required-capability extraction model.
- TLA+ temporal model: accepted-artifact admission and fail-closed run allocation/drive behavior.
- Kani/proptest/fuzz: bounded executable exploration for capability matching, admission checks, and Gate 12 schema.
- Integration/BDD: storage-to-runtime and public API behavior.
- Runtime shell: Fjall, postcard, queues, journal events, and UI serde are verified by integration/roundtrip/fault tests, not Verus.

## Layer assignment

- PRE-001 -> integration + TLA+.
- PRE-002 / INV-003 / POST-002 -> unit + integration + TLA+.
- PRE-003 / INV-004 -> unit + integration + Verus model + serde roundtrip.
- PRE-004 / INV-008 -> API compatibility + integration + BDD.
- PRE-005 / INV-006 / POST-006 / POST-007 -> unit + integration + TLA+.
- PRE-006 / POST-009 -> UI serde roundtrip + integration parity.
- INV-001 / POST-001 -> Kani + proptest + Verus.
- INV-002 / POST-003 / POST-004 -> unit + Kani + Verus.
- INV-005 / POST-005 / POST-008 -> TLA+ + integration journal assertions.
- Release gate -> gauntlet-proof/deep/all after local proof/test artifacts exist.

## Required lanes

- Exact capability unit lane: `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_core capability --lib`.
- Runtime admission unit lane: `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime capability --lib` and `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime admit_artifact_run --lib`.
- No-contract engine lane: `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime without_contract --lib`.
- Gate 12 lane: `TMPDIR=/home/lewis/src/vb-qi37-6/.tmp RUSTC_WRAPPER= cargo test gate_12_contract_capability_validation --test gate_12_14_15_tests` plus stronger diagnostic tests planned.
- Fuzz lanes: `cargo fuzz run capability_name_schema -- -runs=10000` and `cargo fuzz run capability_contract_schema -- -runs=10000`.
- Kani lanes: existing `crates/vb_core/src/kani_capability_harnesses.rs` and `crates/vb_runtime/src/kani_capability_harnesses.rs`; exact harness commands to be confirmed by proof planner/formal verifier before execution.
- TLA lane: planned `tlc -config verification/tla/capability_admission.cfg verification/tla/capability_admission.tla` after proof-writer creates model.
- Verus lane: planned capability model file under `verification/verus/`; exact command blocked until proof-writer creates the file.

## Stale-claim guard

- State 3 contains no proof PASS rows.
- Prior deleted-worktree State 3 summaries are not approval.
- Existing State 1 focused cargo test passes are context only, not proof-obligation execution results.

## Waivers

- Lean waived for now; Verus owns pure capability model unless proof-writing discovers a hard limitation.
- Repo-wide formatting caveat in `baseline-report.md` is not accepted as a State 11 pass and is out of State 3 scope.
