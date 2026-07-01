# Proof Coverage Matrix — vb-t0iw9

schema_version: proof-coverage-matrix/v1
state: 4
bead_id: vb-t0iw9

This matrix maps each planning obligation to its (a) contract clause,
(b) behavior-affecting flag, (c) source artifact binding, (d) default-risk
class, and (e) cross-lane coverage it satisfies. Behavior-affecting rows
must produce a `proof-to-implementation-input.md` bridge row at State 7;
non-behavior rows would not, but the present plan emits no non-behavior
obligation.

## 1. Coverage table

| PO id | requirement_id | contract_clause | verifier | risk | behavior_affecting | source_ref (path::symbol) | bridge row | status |
|---|---|---|---|---|---|---|---|---|
| PO-T0IW9-001 | REQ-T0IW9-001 | contract.md:OB-001 | `proptest` | `parse_canonicalization` | true | `.beads/config.yaml::dolt.server-port` and `/home/lewis/.local/share/mise/installs/go-github-com-steveyegge-beads-cmd-bd/1.0.5/bin/bd::version` | yes -- maps to `DispatchSandboxCapture` | planned |
| PO-T0IW9-002 | REQ-T0IW9-003 | contract.md:OB-003 | `cargo-fuzz` | `hostile_input` | true | `.beads/vb-t0iw9/parsers/SchemaErrorClass_parse.rs::SchemaErrorClass::parse` (planned writer-side artifact path) | yes -- maps to `ReproductionTrace` parser | planned |
| PO-T0IW9-003 | REQ-T0IW9-007 | contract.md:OB-007 | `cargo-fuzz` + `proptest` | `illegal_state` | true | `.beads/config.yaml::dolt.server-port` and `.beads/metadata.json::dolt_database` (closed-key round-trip) | yes -- maps to `EditBeadsConfig` decision | planned |
| PO-T0IW9-004 | REQ-T0IW9-006 | contract.md:OB-006 | `cargo-fuzz` | `rejection` | true | `bd sql "SHOW CREATE TABLE dependencies"::depends_on_id` (STORED-generated parser side) | yes -- maps to `AddSchemaMigrationStatementInvalid` | planned |
| PO-T0IW9-005 | REQ-T0IW9-009 | contract.md:OB-009 | `proptest` | `bounded_transition` | true | `bd supersede vb-qryp7 --with vb-t0iw9::stdout_line_1` | yes -- maps to `VerifyPostRepair` smoke | planned |

Five obligations (in the prompt's "4-6 obligations" bracket). Each row is
behavior-affecting (`true`); the bridge stub (`proof-to-implementation-input.md`)
names all five as required at State 7.

### Notes per obligation

#### PO-T0IW9-001 (proptest, REQ-T0IW9-001 / OB-001)

- claim: `BdVersion` captured from `bd version` is byte-stable across
  repeated probes of the dispatch-sandbox resolution and identical when
  re-probed inside the same femdation child invocation.
- artifact binding: `.beads/config.yaml` is the writable target; the
  bd binary path is read-only.
- behavior-affecting because a stale binary capture invalidates every
  downstream `ReproduceSchemaError` claim.
- anti-invariant: a probe-output mismatch (`expected_digest !=
  actual_digest`) MUST fail the test rather than be silently re-rendered.

#### PO-T0IW9-002 (cargo-fuzz, REQ-T0IW9-003 / OB-003)

- claim: `SchemaErrorClass::parse(raw)` is closed-grammar; hostile
  crafted inputs (empty, multi-line, unicode, truncation, mixed-case)
  reject without panic and route to `Escalate`.
- artifact binding: the writer-side parser under
  `.beads/vb-t0iw9/parsers/SchemaErrorClass_parse.rs`; the production
  contract for the parser is captured in `type-contracts.md § Boundary
  parsers`.
- behavior-affecting because an unparsed raw error string is the only
  signal of dispatch failure.
- anti-invariant: `SchemaErrorClass::parse(invalid_input)` returns
  `SchemaErrorClass::Unclassified { raw_error }` AND never panics; the
  fuzz harness asserts the panic-free property and the unrejectable-class
  property.

#### PO-T0IW9-003 (cargo-fuzz + proptest, REQ-T0IW9-007 / OB-007)

- claim: `BeadsConfig` edits can only target `dolt.server-host` /
  `dolt.server-port`; `BeadsMetadata` edits can only target `dolt_database`
  / `project_id`; the precedence order `BEADS_DOLT_*` → `metadata.json` →
  `config.yaml` is honored and never inverted.
- artifact binding: `.beads/config.yaml` and `.beads/metadata.json`.
- behavior-affecting because a precedence inversion breaks the bd
  config-loader contract.
- anti-invariant: a fuzz corpus with a `BeadsMetadata` carrying a
  `dolt_server_port` key MUST fail to parse; a fuzz corpus with a
  `BeadsConfig` carrying a `dolt_database` key MUST fail to parse.

#### PO-T0IW9-004 (cargo-fuzz, REQ-T0IW9-006 / OB-006)

- claim: `AddSchemaMigration { statement }` parser rejects any
  `ALTER TABLE … DROP COLUMN depends_on_id` or
  `ALTER TABLE … ADD COLUMN depends_on_id … <plain>` statement and emits
  `AddSchemaMigrationStatementInvalid`. The migration chain 0041-0042 is
  intentionally irreversible.
- artifact binding: the writer-side parser under
  `.beads/vb-t0iw9/parsers/AddSchemaMigration_statement.rs`.
- behavior-affecting because a plain-column revival of `depends_on_id`
  breaks the v1.0.5 migration contract.
- anti-invariant: a corpus entry that mentions `depends_on_id` outside a
  STORED/COMMENTED context MUST be rejected as
  `AddSchemaMigrationStatementInvalid`, not normalized away.

#### PO-T0IW9-005 (proptest, REQ-T0IW9-009 / OB-009)

- claim: the post-repair verification re-execution
  (`bd dolt status`, `bd dolt test`, `bd info`, `bd migrate`,
  `bd sql "SHOW COLUMNS FROM dependencies"`,
  `bash scripts/check-beads-server-mode.sh`,
  `bd supersede vb-qryp7 --with vb-t0iw9`) all exit 0 within the documented
  model bounds and produce the documented stdout snippets; the
  `supersede` smoke returns
  `Marked vb-qryp7 as superseded by vb-t0iw9 (closed)`.
- artifact binding: `.beads/vb-t0iw9/post-repair-verification/*.md`
  evidence files (writer produces these).
- behavior-affecting because the verification chain is the only path to
  `PostRepairVerification::Verified`; a false-green breaks the bead's
  closure.
- anti-invariant: an exit code != 0 from any verification command or a
  missing `Marked vb-qryp7 as superseded by vb-t0iw9 (closed)` line MUST
  flip the outcome to `VerificationFailed`, not be silently re-run.

## 2. Risk-class coverage check

Each obligation contributes to at least one default-risk-class coverage
target. The check is repeated from `verifier-lane-matrix.md §3` for
audit-trail:

- `hostile_input` → PO-T0IW9-002 ✓, PO-T0IW9-003 ✓, PO-T0IW9-004 ✓.
- `parse_canonicalization` → PO-T0IW9-001 ✓, PO-T0IW9-002 ✓.
- `rejection` → PO-T0IW9-002 ✓, PO-T0IW9-004 ✓.
- `illegal_state` → PO-T0IW9-003 ✓.
- `bounded_transition` → PO-T0IW9-005 ✓.

## 3. Out-of-coverage risks and rationale

The following default-risk classes are not raised by this bead; they
appear in `verifier-lane-decisions.jsonl` as `applicability: not_applicable`
with concrete limitation_kind and evidence refs (see
`verifier-lane-matrix.md §4`):

- `arithmetic_overflow`, `index_safety`, `panic_freedom`:
  `limitation_kind: surface_absent` (no production Rust to bound).
- `refinement`: same.
- `concurrency_interleaving`, `cancellation_safety`, `shutdown_drain`:
  `limitation_kind: risk_out_of_scope` (`domain-model.md §56-60` shows
  no concurrency concerns).
- `temporal_liveness`, `temporal_safety`: same (workflow-model.md §93-97
  covers the workflow state machine via `bounded_transition` covered by
  PO-T0IW9-005; the temporal hazards noted there are read-only and
  deterministic, so they fold into `bounded_transition`).
- `ub_safety`: `limitation_kind: surface_absent` (no `unsafe`).

No waiver row is required to bridge these `not_applicable` decisions; the
typed limitation_kind + non-empty evidence refs are sufficient under the
validator policy.

## 4. Bridge to implementation

The State-7 `proof-to-implementation-input.md` stub enumerates each of
the five obligations as a required bridge row, with:

- `source_refs` populated from §1 column "source_ref" (using
  `path::symbol` form).
- `behavior_test_refs` populated from the writer-side parser path or the
  existing CLIs (for the integration-test row).
- `refinement_harness_refs` populated from the cargo-fuzz target list
  (for the hostile-input rows) and the proptest harness list (for the
  bounded-state row).
- `evidence_command` populated from each obligation's `command` field.
- `mapping_status: planned` (State 7 materializes this into
  `rust-refinement-obligation/v1`).
