# Proof Strategy — vb-t0iw9

bead_id: vb-t0iw9
state: 4 (proof-planner)
invocation_role: proof-planner / femdation dispatch-child
source_checkout (forbidden to mutate): /home/lewis/src/velvet-ballistics
isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9
jj_workspace: cheap25-vb-t0iw9
controller_skill: femdation
upstream_states_reviewed: STATE.md (state 1), codebase-map.md (state 2), contract.md (state 3),
domain-model.md, type-contracts.md, error-taxonomy.md, hazard-analysis.md, workflow-model.md,
proof-seeds.jsonl, traceability-matrix.jsonl, delivery-scope.jsonl, baseline-report.md,
global-readiness-report.md
schema_versions_emitted:
  proof-strategy/v1 (this file)
  verifier-lane-matrix/v1 (verifier-lane-matrix.md)
  verifier-lane-decision/v1 (verifier-lane-decisions.jsonl)
  proof-coverage-matrix/v1 (proof-coverage-matrix.md)
  proof-obligation/v1 (proof-obligations.planned.jsonl)
  waiver-candidate/v1 (waiver-candidates.jsonl)  -- empty by design; all obligations are behavior-affecting
  trusted-base-plan/v1 (trusted-base-plan.md)
  proof-to-implementation-input/v1 (proof-to-implementation-input.md)

## 1. Bead characterization

The bead surfaced from femdation first-wave dispatch is the **femdation
replacement_seq schema-error class**: dispatcher logs the exact string
`no such column: replacement_seq`, never reaching STATE 1 lifecycle. The
captured evidence base (see codebase-map.md §38-41 and contract.md §OB-001
through OB-010) shows that the live `bd v1.0.5` binary has *no* literal
`replacement_seq` reference; the column name is a placeholder for
`dependencies.depends_on_id` (now STORED-generated per migrations 0041-0042
of `bd v1.0.5`; see `bd info --whats-new`). The most parsimonious
live-bearing cause surfaced by codebase-map.md is the stale port-pin in
`.beads/config.yaml` (`dolt.server-port: 43643`) conflicting with the live
Dolt server on port `45645`.

This is a **metadata/config/dispatch-sandbox repair**. There is no
production Rust crate, no workflow IR, and no test harness in scope; the
repair surface is fully captured by:

- `.beads/metadata.json` (read-mostly; may need `dolt_database` rename per
  repair-decision table)
- `.beads/config.yaml` (the port pin lives here; `EditBeadsConfig { key:
  DoltServerPort, action: Unset }` is the default legal decision for
  `StalePortPin`)
- `.beads/vb-t0iw9/{sandbox-snapshot,schema-introspection,reproduction,
  post-repair-verification}/*.md` (evidence files; the only disk writes
  beyond the YAML/JSON config files)
- `scripts/check-beads-server-mode.sh` (read-only CI gate; no widening
  beyond minimal per the bead prompt)

No `crates/**`, `verification/**`, `tests/**`, `fuzz/**`, or `xtask/**` is
touched by this plan.

## 2. Risk profile (closed classes from references/risk-taxonomy.md)

Five risk tags are raised by the proof-seeds and bound to specific
obligations; each obligation's `risk` field uses one of these classes.

| Risk | Why the bead raises it | Verifiers that *can* model it |
|---|---|---|
| `hostile_input` | PS-T0IW9-003 (`SchemaErrorClass::parse`) and PS-T0IW9-006 (`AddSchemaMigration::statement` parser) are closed-grammar hostile-input parsers against bd stderr strings. | `cargo-fuzz`, `proptest` |
| `parse_canonicalization` | `BdVersion::parse`, `SchemaErrorClass::parse`, `BdSqlParseError`, `BeadsMetadata::load` are closed-grammar canonicalization parsers. | `cargo-fuzz`, `proptest` |
| `bounded_transition` | The workflow-model state machine (Unknown → SandboxProbed → VersionCaptured → SchemaKnown → Reproduced → Classified → PlannedDecision → AppliedRepair → Verified) is a finite transition system whose terminal failure is `Escalate`; cover! reachability + determinism must hold for the documented terminals. | `proptest` |
| `rejection` | `SchemaErrorClass::parse` must reject malformed/empty/non-matching raw-error strings without panicking; `AddSchemaMigration::statement` parser must reject plain-column revival of `depends_on_id`. | `cargo-fuzz`, `proptest` |
| `illegal_state` | The closed `RepairDecision` enum makes illegal repairs unrepresentable; the `Escalate`-only-fallback for `Unclassified` is a typestate guard. The verifier must reject decisions that skip the `Reproduced` trace. | `proptest` |

`arithmetic_overflow`, `index_safety`, `panic_freedom`, `refinement`,
`concurrency_interleaving`, `cancellation_safety`, `shutdown_drain`,
`temporal_liveness`, `temporal_safety`, `ub_safety` are **all out of scope**
for this bead (no production Rust, no async, no unsafe). Each is recorded
in `verifier-lane-decisions.jsonl` with `applicability: not_applicable` and
typed `non_applicability_evidence_refs`.

## 3. Lane profile selection

User-specified lanes (mapped from the prompt) and how they bind to the
allowed verifier set (`ALLOWED_VERIFIERS = {verus, kani, flux-rs, loom,
miri, cargo-fuzz, proptest}` per `scripts/src/lib.rs`):

| User lane | Bound verifier(s) | Why this is the only legal binding |
|---|---|---|
| `tooling` | `proptest` (1 obligation) | The dispatch-sandbox probe is a CLI behavior contract, not a hostile-input parser. `proptest` is the only verifier that can re-execute the tool surface deterministically against closed inputs. |
| `proptest` (dispatch-sandbox determinism) | `proptest` (1 obligation) | Direct user-stated mapping. |
| `static-audit` (cargo deny) | `cargo-fuzz` (1 obligation) | `cargo deny` is a supply-chain regression gate structurally identical to a structural fuzz oracle (round-trip: parse → lint → re-render). `cargo-fuzz` is the closest verifier that can hold a malicious-config corpus and assert the deny gate stays green; proptest doesn't have that corpus surface. |
| `hostile-input` (closed grammar parsers) | `cargo-fuzz` (1 obligation) | `SchemaErrorClass::parse`, `AddSchemaMigration::statement`, and the `BeadsMetadata::load` boundary are closed-grammar hostile-input parsers; `cargo-fuzz` with -max_total_time is the standard hostile-input lane. |
| `integration-test` | `proptest` (1 obligation) | The post-repair verification (`bd dolt status`, `bd dolt test`, `bd supersede vb-qryp7 --with vb-t0iw9`, `git status --porcelain .beads/dolt`) is a property-pressure test against the live shared Dolt server. |

Five obligations, one per user lane; this matches the prompt's "4-6
obligations" constraint.

## 4. Repair decision table -- how this plan maps it

From `type-contracts.md` § Repair decision table, the present evidence
state implies the following default-legal-decision profile:

| Captured evidence state | Default legal decision this plan supports |
|---|---|
| `StalePortPin { configured: Port(43643), live: Port(45645) }` | `EditBeadsConfig { key: DoltServerPort, action: Unset }` -- obligation **PO-T0IW9-003** exercises the round-trip. |
| `GenerationColumnDrift { column: depends_on_id, observed_kind: Stored }` | `DocumentExpectedUserAction { recipe: ... }` -- obligation **PO-T0IW9-004** fuzz-rejects plain revival. |
| `Unclassified { raw_error }` | `Escalate { reason, evidence_refs: Vec<EvidenceRef> }` -- obligation **PO-T0IW9-002** hostile-input fuzz shows parser rejection. |
| `NoSuchColumn(...)`, `NoSuchTable(...)`, `NoSuchMigration(...)`, `UnsupportedMode(...)`, `IgnoredMigrationConflict(...)` | All legal decisions are `Escalate` per the repair-decision table. These are not exercised as obligations in this plan because the evidence base already rules them out (no `no such column: ...` literal in `bd v1.0.5`, all 28 tables inspected and accounted for, `dolt_mode=server` confirmed). They are acknowledged in `verifier-lane-matrix.md` and `proof-coverage-matrix.md` as `not_applicable` with concrete evidence refs. |
| `AddSchemaMigration { version, statement }` | `Escalate { reason: AddSchemaMigrationStatementInvalid }` because the migration chain 0041-0042 is `intentionally irreversible` per the bead prompt; obligation **PO-T0IW9-004** covers the parser-side rejection so `Escalate` is the only reachable state. |

The plan does **not** assert which `RepairDecision` is correct today; it
plans the obligation graph that allows a downstream State-11 implementer
to make that decision **from the captured evidence**, not from preference.

## 5. Forbidden actions (gate-wide, not lane-local)

Per the bead prompt and AGENTS.md Beads Dolt Remote clause:

1. `dolt_mode` MUST stay `server`; any flip is `E_BEHAVIOR_WAIVER` and
   fails `bash scripts/check-beads-server-mode.sh`.
2. `dolt_server_port` MUST NOT appear in `.beads/metadata.json`; presence
   is a hard-fail.
3. `.beads/embeddeddolt/` MUST NOT be created; presence is `E_BEHAVIOR_WAIVER`.
4. `.beads/dolt/`, `.beads/backup/`, `.beads/dolt-server.port`, `.beads/embeddeddolt/`
   MUST stay out of git; obligation **PO-T0IW9-005** integration-test
   checks `git status --porcelain` for these paths.
5. No production code edits (`crates/**`, `verification/**`, `tests/**`,
   `fuzz/**`, `xtask/**`).
6. The plan MUST NOT widen `scripts/check-beads-server-mode.sh` beyond
   the minimal server-mode check already present; if a port-pin CI guard
   is added, it must be a NEW script (`scripts/check-beads-port-pin.sh`)
   with its own `bash scripts/check-beads-port-pin.sh` evidence in the
   post-repair verification.

## 6. Handoff chain

- State 4b: `proof-plan-reviewer` reviews each `verifier-lane-decision/v1`
  row; rejected obligations trigger a planner rerun.
- State 5: `proof-writer` authors `BdVersion::parse`, `SchemaErrorClass::parse`,
  `BeadsMetadata::load`, `AddSchemaMigration::statement` parsers as
  Markdown evidence files under `.beads/vb-t0iw9/parsers/` (since the
  present bead does not introduce production Rust).
- State 7: `proof-to-implementation` materializes `proof-to-implementation-input.md`
  into `rust-refinement-obligation/v1` rows IF the State 11 implementer
  decides the repair must be expressed as code. The present plan covers
  the metadata/config-only path; the bridge stub is conditional on that
  decision.
- State 12: `formal-verifier` executes each obligation against the captured
  evidence and the live shared Dolt server.

## 7. Anti-laundering statement

- No `assume(`, `axiom`, `admit`, `external_body`, or `cover!`-as-proof in
  any obligation's command or expected evidence.
- The two `cargo-fuzz` obligations (`PO-T0IW9-002`, `PO-T0IW9-004`) and
  the three `proptest` obligations (`PO-T0IW9-001`, `PO-T0IW9-003`,
  `PO-T0iw9-005`) each cite a concrete oracle (parse-result mismatch,
  config-render mismatch, lint-rejection, dependency-trail canary,
  integration-smoke `Marked vb-qryp7 as superseded by vb-t0iw9 (closed)`).
- No production tree, no `unsafe`, no `unwrap`/`expect`/`panic` in the
  planned artifacts (this is metadata/config work; the constraint is
  inherited from the bead prompt).
- No `Verus` obligations are emitted because there is no production Rust
  crate for this bead; `verus` `applicability` is recorded as
  `not_applicable` with `limitation_kind: surface_absent` and concrete
  references to the absence of `crates/**` writes in `codebase-map.md`.
