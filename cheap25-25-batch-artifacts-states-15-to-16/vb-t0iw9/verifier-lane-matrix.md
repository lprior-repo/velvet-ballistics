# Verifier Lane Matrix — vb-t0iw9

schema_version: verifier-lane-matrix/v1
state: 4
bead_id: vb-t0iw9
controller: femdation
allowed_verifier_set: [verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest]
default_profile_required_lanes: derived from references/risk-taxonomy.md →
DEFAULT_RISK_PROFILE in scripts/src/lib.rs.

## 1. Per-seed lane profile

Each row is one (requirement_id, contract_clause, proof_seed_id) tuple and
the lane profile the planner asserts for it. Lane decisions are emitted as
`verifier-lane-decision/v1` rows in `verifier-lane-decisions.jsonl`.

### 1.1 PS-T0IW9-001 / REQ-T0IW9-001 / OB-001 (dispatch-sandbox capture)

- risk: `parse_canonicalization` (canonical `BdVersion` from `bd version`)
- user lane binding: `tooling`
- required verifiers: `proptest`
- rationale: `BdVersion::parse` is a closed-grammar parser over `bd version`
  output; `proptest` over repeated probes is the only binding that produces
  deterministic reproducibility of the capture script.
- non-required verifiers: `cargo-fuzz` (hostile-input variant not needed --
  the capture sandbox is a trusted-host surface, not an attacker surface).

### 1.2 PS-T0IW9-002 / REQ-T0IW9-002 / OB-002 (schema introspection)

- risk: `bounded_transition` (read-only state-machine against the live
  shared Dolt server at 127.0.0.1:45645)
- user lane binding: `integration-test`
- required verifiers: `proptest`
- rationale: each `bd dolt status`/`bd sql` call is a property-pressure
  test against the live server; `proptest` with repeated-sample
  model_bounds is the only verifier that carries the live-snapshot
  surface.
- non-applicable lanes: `cargo-fuzz` (corpus doesn't apply; introspection
  surface is read-only), `verus`/`kani`/`flux-rs` (no production Rust in
  this bead).

### 1.3 PS-T0IW9-003 / REQ-T0IW9-003 / OB-003 (reproduction + error
classification)

- risk: `hostile_input` + `parse_canonicalization` (closed-grammar parser
  for `SchemaErrorClass::parse`)
- user lane binding: `hostile-input`
- required verifiers: `cargo-fuzz`, `proptest`
- rationale: `cargo-fuzz` carries the hostile-input corpus for malformed
  raw-error strings; `proptest` carries the structured-property version
  (every `BeadsConfigParseError`/`BeadsMetadataParseError` shape must
  reject without panicking).

### 1.4 PS-T0IW9-004 / REQ-T0IW9-004 / OB-004 (repair decision selection)

- risk: `illegal_state` (closed `RepairDecision` enum)
- user lane binding: `hostile-input`
- required verifiers: `cargo-fuzz`, `proptest`
- rationale: `cargo-fuzz` exercises the `AddSchemaMigration::statement`
  parser with adversarial `depends_on_id` statements; `proptest` exercises
  the `RepairDecision` decision table exhaustively across the closed
  `SchemaErrorClass` × `ReproductionTrace` cross product.

### 1.5 PS-T0IW9-005 / REQ-T0IW9-005 / OB-005 (server-mode preservation)

- risk: `illegal_state`
- user lane binding: `static-audit` (cargo-deny-style gate)
- required verifiers: `cargo-fuzz`, `proptest`
- rationale: `cargo-fuzz` runs the cargo-deny-style regression corpus
  against `BeadsMetadata::load` (round-trip: parse → lint → re-render);
  `proptest` carries the property-pressure variant.
- forbidden mutation: presence of `dolt_mode=embedded`, `dolt_server_port`
  key, or `.beads/embeddeddolt/` directory is the anti-invariant for the
  fuzz harness; presence MUST cause a parser rejection.

### 1.6 PS-T0IW9-006 / REQ-T0IW9-006 / OB-006 (STORED-column respect)

- risk: `rejection` (the `AddSchemaMigration::statement` parser must
  reject plain-column revival of `depends_on_id`)
- user lane binding: `hostile-input`
- required verifiers: `cargo-fuzz`
- rationale: `cargo-fuzz` alone covers the closed-grammar hostile-input
  parser; `proptest` is non-required (and would duplicate effort).

### 1.7 PS-T0IW9-007 / REQ-T0IW9-007 / OB-007 (config precedence)

- risk: `illegal_state` (closed `ConfigKey`/`MetadataKey` enums)
- user lane binding: `static-audit`
- required verifiers: `cargo-fuzz`, `proptest`
- rationale: `cargo-fuzz` runs the precedence corruption corpus
  (privilege-escalation attempts where `BeadsMetadata` carries a
  `dolt_server_port` key or where `BeadsConfig` carries a `dolt_database`
  key); `proptest` carries the structural-property version.

### 1.8 PS-T0IW9-008 / REQ-T0IW9-008 / OB-008 (git-cleanliness)

- risk: `bounded_transition` (read-only git state-machine)
- user lane binding: `integration-test`
- required verifiers: `proptest`
- rationale: `git status --porcelain` is a deterministic CLI surface; a
  property-pressure test against `.beads/dolt`/`.beads/backup`/
  `.beads/dolt-server.port` paths is the only required verifier.

### 1.9 PS-T0IW9-009 / REQ-T0IW9-009 / OB-009 (post-repair verification)

- risk: `bounded_transition` (verification re-execution)
- user lane binding: `integration-test`
- required verifiers: `proptest`
- rationale: each verification command exits 0 with documented stdout;
  `proptest` with deterministic-seed model_bounds carries repeated
  re-execution.
- co-vary: the `bd supersede vb-qryp7 --with vb-t0iw9` smoke output
  `Marked vb-qryp7 as superseded by vb-t0iw9 (closed)` is the
  documentation of the live evidence; the oracle for the smoke is the
  *first line of stdout exactly matching* that phrase.

### 1.10 PS-T0IW9-010 / REQ-T0IW9-010 / OB-010 (failure routing → Escalate)

- risk: `bounded_transition` (terminal-state routing)
- user lane binding: `integration-test`
- required verifiers: `proptest`
- rationale: the workflow state machine is closed; `proptest` exhausts
  the terminal-state matrix and asserts that any failure path routes to
  `Escalate(reason, evidence_refs)` with `evidence_refs.len() >= 3`.

## 2. Cross-lane coverage summary

| Verifier | Required count | Required seeds |
|---|---|---|
| `cargo-fuzz` | 4 | PS-T0IW9-003, PS-T0IW9-004, PS-T0IW9-005, PS-T0IW9-006, PS-T0IW9-007 |
| `proptest`    | 5 | PS-T0IW9-001, PS-T0IW9-002, PS-T0IW9-003, PS-T0IW9-004, PS-T0IW9-005, PS-T0IW9-007, PS-T0IW9-008, PS-T0IW9-009, PS-T0IW9-010 |

(Each required obligation sits on one seed; the cross-lane coverage
differs by seed. The planner picks five obligations covering five distinct
seeds to honour the prompt's "4-6 obligations" constraint.)

## 3. Required profiler decision by risk class

Derived from `DEFAULT_RISK_PROFILE` in `scripts/src/lib.rs` and
`references/risk-taxonomy.md`:

| Risk class | Required verifiers per profile | Compliance in this plan |
|---|---|---|
| `hostile_input`        | `cargo-fuzz`, `kani`, `proptest` | `cargo-fuzz` ✓ (PO-T0IW9-002, PO-T0IW9-004); `proptest` ✓ (PO-T0IW9-003, PO-T0IW9-005); `kani` N/A: `limitation_kind=surface_absent` (no production Rust under hostile-input target; recorded in VLD-N/A-001). |
| `parse_canonicalization` | `cargo-fuzz`, `verus`, `kani` | `cargo-fuzz` ✓ (PO-T0IW9-002, PO-T0IW9-004); `verus`/`kani` N/A: `limitation_kind=surface_absent` (no production Rust crate; recorded in VLD-N/A-002). |
| `bounded_transition`     | `kani`, `verus`              | `proptest` carries the in-bead equivalent; `kani`/`verus` N/A: `limitation_kind=surface_absent` (no production Rust; recorded in VLD-N/A-003). |
| `rejection`              | `kani`, `proptest`           | `cargo-fuzz` ✓ (PO-T0IW9-004); `proptest` ✓ (PO-T0IW9-004); `kani` N/A: `limitation_kind=surface_absent`. |
| `illegal_state`          | `flux-rs`, `verus`           | `proptest`/`cargo-fuzz` ✓; `flux-rs`/`verus` N/A: `limitation_kind=surface_absent`. |

Each `not_applicable` row in `verifier-lane-decisions.jsonl` cites the
specific limitation class, the bead-specific reason (no production Rust),
and at least one evidence ref pointing at either
`codebase-map.md §71` (explicit out-of-scope call) or
`delivery-scope.jsonl:14` (`touched_crates: []`).

## 4. Non-applicable lanes (closure summary)

The verifier set has seven entries; this plan requires `proptest` and
`cargo-fuzz` only. The remaining five (`verus`, `kani`, `flux-rs`, `loom`,
`miri`) are emitted as `applicability: not_applicable` with typed
`non_applicability_evidence_refs`:

| Verifier | Limitation | Evidence ref |
|---|---|---|
| `verus`   | `surface_absent`                       | `codebase-map.md §71` ("Excluded paths ... `crates/**` `verification/**` `tests/**` `fuzz/**` `xtask/**` ... out of scope. This bead targets the beads-tracker (bd) and femdation orchestrator surface, not application code."); `delivery-scope.jsonl:touched_crates` is `[]`. |
| `kani`    | `surface_absent`                       | Same as above. Kani requires `#[kani::proof]` harnesses; no production Rust exists for this bead. |
| `flux-rs` | `surface_absent`                       | Same as above. Flux requires `#![flux::cfg]` refined sources; this bead has no `.rs` production files to refine. |
| `loom`    | `risk_out_of_scope`                    | `domain-model.md §56-60` ("Open domain decisions" lists no concurrency concerns). `codebase-map.md §71` ("out of scope"). The dispatch-sandbox is single-threaded CLI; no async boundary to model. |
| `miri`    | `surface_absent`                       | Same as above. Miri requires `unsafe Rust`; this bead has no Rust code at all. |
| `tla-plus` (legacy) | `surface_absent`             | Skill policy: `proof-planner/ALLOWED_VERIFIERS` does not include `tla-plus`; temporal workflows are covered by `loom`+`proptest` per SKILL.md "TLA+ removed" clause. |

Each row is emitted as a `verifier-lane-decision/v1` row with
`applicability: not_applicable`, `limitation_kind` set, and
`non_applicability_evidence_refs` non-empty. None are silently omitted.
