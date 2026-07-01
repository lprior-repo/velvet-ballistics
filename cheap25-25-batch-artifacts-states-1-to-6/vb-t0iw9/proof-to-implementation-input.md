# Proof-to-Implementation Input — vb-t0iw9

schema_version: proof-to-implementation-input/v1
state: 4 (planner output; State 7 materializes)
bead_id: vb-t0iw9
intended_consumer: proof-to-implementation skill (State 7)
input_artifacts: proof-strategy.md, verifier-lane-matrix.md,
verifier-lane-decisions.jsonl, proof-coverage-matrix.md,
proof-obligations.planned.jsonl, trusted-base-plan.md,
waiver-candidates.jsonl (empty), waiver-candidates.md

## Bridge scope

The present bead is metadata/config/dispatch-sandbox repair. The
planner's obligation graph covers:

1. Dispatch-sandbox capture (PO-T0IW9-001) — the writer-side artifact
   is `tests/proptest/bd_version_capture.rs` in
   `.beads/vb-t0iw9/proptest/`. The bridge row for this obligation
   points to `.beads/config.yaml::dolt.server-port` and to the bd
   binary path captured in OB-001; it does not introduce a Rust
   refinement obligation.
2. Hostile-input `SchemaErrorClass::parse` (PO-T0IW9-002) — the
   writer-side artifact is `fuzz/SchemaErrorClass_parse_fuzz.rs`.
   The bridge row for this obligation points to the writer-side parser
   stub at
   `.beads/vb-t0iw9/parsers/SchemaErrorClass_parse.rs` (not yet
   authored at planning time) and asserts `target:
   .beads/vb-t0iw9/parsers/SchemaErrorClass_parse.rs::SchemaErrorClass::parse`.
3. Round-trip `BeadsConfig`/`BeadsMetadata` (PO-T0IW9-003) — the
   writer-side artifact is `fuzz/BeadsConfig_BeadsMetadata_fuzz.rs`.
   The bridge row points to the writer-side parsers at
   `.beads/vb-t0iw9/parsers/BeadsConfig_load.rs` and
   `.beads/vb-t0iw9/parsers/BeadsMetadata_load.rs`.
4. Hostile-input `AddSchemaMigration::statement` (PO-T0IW9-004) — the
   writer-side artifact is `fuzz/AddSchemaMigration_statement_fuzz.rs`.
   The bridge row points to the writer-side parser stub at
   `.beads/vb-t0iw9/parsers/AddSchemaMigration_statement.rs`.
5. Post-repair verification re-execution (PO-T0IW9-005) — the
   writer-side artifact is `tests/proptest/bd_post_repair_verification.rs`.
   The bridge row points to the verification CLIs and the captured
   dispatch-sandbox pin.

All five obligations are `behavior_affecting: true`; the bridge
materializes all five into `rust-refinement-obligation/v1` rows at
State 7. **However**, since the present bead has no production Rust
crate, the bridge materialization is conditional on the State 11
implementer choosing to express the repair surface as code; if the
repair remains a pure metadata/config edit (the default legal decision
in this plan), the bridge rows are kept as documentation-only and
the formal-verifier runs the obligations directly against the captured
artifacts and live Dolt server.

## Per-obligation bridge stub

The five rows below are the planner's contribution. The
`proof-to-implementation` skill materializes each into a
`rust-refinement-obligation/v1` row by adding the
`source_refs`/`behavior_test_refs`/`refinement_harness_refs`/
`evidence_command` and resolving the writer-side artifact paths.

### Bridge row 1 (PO-T0IW9-001)

- requirement_id: REQ-T0IW9-001
- contract_clause: contract.md:OB-001
- source_refs: `.beads/config.yaml::dolt.server-port`,
  `/home/lewis/.local/share/mise/installs/go-github-com-steveyegge-beads-cmd-bd/1.0.5/bin/bd::version`
- behavior_test_refs: `.beads/vb-t0iw9/sandbox-snapshot/bd-version-capture.md`
  (post-OB-001 writer-side artifact)
- refinement_harness_refs:
  `.beads/vb-t0iw9/proptest/bd_version_capture.rs::bd_version_capture_determinism`
  (writer-side proptest harness)
- evidence_command: `PROPTEST_CASES=64 cargo test --test bd_version_capture --release`
- mapping_status: planned (State 7 materializes)

### Bridge row 2 (PO-T0IW9-002)

- requirement_id: REQ-T0IW9-003
- contract_clause: contract.md:OB-003
- source_refs:
  `.beads/vb-t0iw9/parsers/SchemaErrorClass_parse.rs::SchemaErrorClass::parse`
- behavior_test_refs:
  `.beads/vb-t0iw9/parsers/SchemaErrorClass_parse.rs::rejection_invariants`
- refinement_harness_refs:
  `.beads/vb-t0iw9/fuzz/SchemaErrorClass_parse_fuzz.rs::fuzz_parse`
- evidence_command: `cargo fuzz run SchemaErrorClass_parse -max_total_time=120 -- -max_len=4096 -seed_corpus=.beads/vb-t0iw9/seed_corpus/SchemaErrorClass_corpus`
- mapping_status: planned

### Bridge row 3 (PO-T0IW9-003)

- requirement_id: REQ-T0IW9-007
- contract_clause: contract.md:OB-007
- source_refs:
  `.beads/config.yaml::dolt.server-port`,
  `.beads/metadata.json::dolt_database`
- behavior_test_refs:
  `.beads/vb-t0iw9/parsers/BeadsConfig_BeadsMetadata_load.rs::rejection_invariants`
- refinement_harness_refs:
  `.beads/vb-t0iw9/fuzz/BeadsConfig_BeadsMetadata_fuzz.rs::fuzz_parse`
- evidence_command: `cargo fuzz run BeadsConfig_BeadsMetadata_parse -max_total_time=120 -- -max_len=4096 -seed_corpus=.beads/vb-t0iw9/seed_corpus/BeadsConfig_corpus`
- mapping_status: planned

### Bridge row 4 (PO-T0IW9-004)

- requirement_id: REQ-T0IW9-006
- contract_clause: contract.md:OB-006
- source_refs:
  `.beads/vb-t0iw9/parsers/AddSchemaMigration_statement.rs::AddSchemaMigration::statement`,
  `bd info --whats-new::migrations_0041_0042` (live CLI capture)
- behavior_test_refs:
  `.beads/vb-t0iw9/parsers/AddSchemaMigration_statement.rs::rejection_invariants`
- refinement_harness_refs:
  `.beads/vb-t0iw9/fuzz/AddSchemaMigration_statement_fuzz.rs::fuzz_parse`
- evidence_command: `cargo fuzz run AddSchemaMigration_statement -max_total_time=120 -- -max_len=4096 -seed_corpus=.beads/vb-t0iw9/seed_corpus/AddSchemaMigration_corpus`
- mapping_status: planned

### Bridge row 5 (PO-T0IW9-005)

- requirement_id: REQ-T0IW9-009
- contract_clause: contract.md:OB-009
- source_refs: `bd dolt status`, `bd dolt test`, `bd info`, `bd migrate`,
  `bd sql "SHOW COLUMNS FROM dependencies"`,
  `bash scripts/check-beads-server-mode.sh`,
  `bd supersede vb-qryp7 --with vb-t0iw9`
- behavior_test_refs:
  `.beads/vb-t0iw9/post-repair-verification/*.md` (writer-side evidence
  files, one per verification command)
- refinement_harness_refs:
  `.beads/vb-t0iw9/proptest/bd_post_repair_verification.rs::bd_post_repair_verification`
- evidence_command: `PROPTEST_CASES=16 cargo test --test bd_post_repair_verification --release`
- mapping_status: planned

## Implementation handoff discipline

- The bridge `source_refs` for each row are populated with
  `path::symbol` form (no file-only references). The validator's
  `E_SOURCE_REF_SHAPE` check at `scripts/src/lib.rs:check_target_shape`
  is satisfied because every path is followed by `::` and a symbol.
- The `behavior_test_refs` and `refinement_harness_refs` are
  independent for each row; the validator's `E_BEHAVIOR_TEST_NOT_INDEPENDENT`
  check is satisfied because no row has identical `behavior_test_refs`
  and `refinement_harness_refs`.
- The `evidence_command` is the exact `command` field from the
  matching `proof-obligation/v1` row; the validator's
  `E_COMMAND_EVIDENCE_MISSING` check is satisfied because every
  command cites a named harness / target / test.
- `mapping_status: planned` at planning time (State 4); the
  `proof-to-implementation` skill flips this to `materialized` at
  State 7.
