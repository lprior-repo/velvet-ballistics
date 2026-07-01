# Verifier Lane Matrix — vb-d9ml3 (Storage trim/snapshot key length cap, P1)

> Schema companion to `verifier-lane-decisions.jsonl`. This matrix is the
> human-readable binding for the schema-level `verifier-lane-decision/v1`
> rows. The schema-version, id, and decision-reason fields are authoritative
> in the JSONL; this Markdown mirrors them and adds the per-seed
> `(requirement_id, contract_clause, proof_seed_id, verifier)` tuple plus
> the rationale for `required` vs. `not_applicable`.

Bead ID: `vb-d9ml3`
Planner invocation: `proof-planner-vb-d9ml3-state4`
Workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3`
Owner state: 4
Captured: 2026-07-01

---

## Lane profile summary

| Lane | Status | Total rows | Required PO IDs |
|---|---|---:|---|
| `default_rust_lane` (unit) | required | 2 | PO-001-UNIT, PO-001-REGRESSION |
| `proptest` (length roundtrip) | required | 1 | PO-003-PROPTEST |
| `integration` (existing planted-malformed-key tests) | required | 1 | PO-002-INTEGRATION |
| `moon-source-lint` (lint + workspace) | required | 1 | PO-004-LINT |
| `kani` | not_applicable | 1 | (none — proof-writer does not author a Kani harness) |
| `verus` | not_applicable | 1 | (none — proof-writer does not author a Verus spec) |
| `flux-rs` | not_applicable | 1 | (none — proof-writer does not author a Flux refinement) |
| `cargo-fuzz` | not_applicable | 1 | (none — proof-writer does not author a fuzz harness) |
| `loom` | not_applicable | 1 | (none — proof-writer does not author a Loom model) |
| **Total** | | **9** | **4 PO rows** |

The schema enforces `verifier` ∈ `{verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest}`.
The `default_rust_lane (unit)`, `integration`, and `moon-source-lint` lanes are
all routed through the `proptest` verifier vocabulary in the schema because
(a) the schema has no separate `unit` / `cargo-test` / `clippy` verifier, and
(b) the contract (CC-CAP-001..010) commits to these lanes as canonical Rust
discipline (Holzman Rust). The cross-lane distinction is preserved in the
`decision_reason` and `expected_evidence` fields of each obligation.

---

## Required lane rows

### VLD-001 — `default_rust_lane (unit) / const-equality` ⇒ `proptest` verifier

| Field | Value |
|---|---|
| `id` | `VLD-001` |
| `requirement_id` | `REQ-CAP-001` |
| `contract_clause` | `CC-CAP-001` |
| `proof_seed_id` | `PS-CAP-CONST-001` |
| `verifier` | `proptest` |
| `risk_tags` | `["public_api", "numeric/cap_refinement", "equality"]` |
| `applicability` | `required` |
| `decision_reason` | CC-CAP-001 requires a const-alias equality unit test; the schema has no separate `unit` verifier, so the lane is routed through `proptest` with a degenerate strategy (`PROPTEST_CASES=10`, `prop_assert_eq!(MAX_TRIM_KEY_LEN, JOURNAL_KEY_BYTES)` and `prop_assert_eq!(MAX_SNAPSHOT_KEY_LEN, JOURNAL_KEY_BYTES)`). The literal `17` is replaced by `JOURNAL_KEY_BYTES` at the alias site, so the const chain enforces compile-time equality. |
| `required_obligation_ids` | `["PO-001-UNIT"]` |
| `non_applicability_evidence_refs` | `[]` |
| `limitation_kind` | `""` |

### VLD-002 — `default_rust_lane (unit) / variant preservation` ⇒ `proptest` verifier

| Field | Value |
|---|---|
| `id` | `VLD-002` |
| `requirement_id` | `REQ-CAP-005` |
| `contract_clause` | `CC-CAP-005` |
| `proof_seed_id` | `PS-CAP-UNIT-004` |
| `verifier` | `proptest` |
| `risk_tags` | `["public_api", "error_taxonomy", "equality"]` |
| `applicability` | `required` |
| `decision_reason` | CC-CAP-005 requires the existing test `journal_error_trim_wrapper_delegates_incomplete_trim_code` at `error_code_tests.rs:~246` to continue to assert the `0x4102` propagation; the proof strategy pins this test as a regression gate via the `proptest` verifier vocabulary (cargo test invocation). The diagnostic code is `INCOMPLETE_TRIM_CODE` (line 62 of `trimming/mod.rs`); `JournalError::Trim(inner).diagnostic_code()` delegates to `inner.diagnostic_code()` (`error/codes.rs:167`). |
| `required_obligation_ids` | `["PO-001-REGRESSION"]` |
| `non_applicability_evidence_refs` | `[]` |
| `limitation_kind` | `""` |

### VLD-003 — `proptest / length roundtrip` ⇒ `proptest` verifier

| Field | Value |
|---|---|
| `id` | `VLD-003` |
| `requirement_id` | `REQ-CAP-002` |
| `contract_clause` | `CC-CAP-002` |
| `proof_seed_id` | `PS-CAP-PROPTEST-001` |
| `verifier` | `proptest` |
| `risk_tags` | `["parser/codec", "bounded_state", "hostile_input", "rejection", "property"]` |
| `applicability` | `required` |
| `decision_reason` | CC-CAP-002/003/004 demand an arbitrary-length key generator over the snapshot prefix. The proptest exercises the full length space `0..=256` and asserts the only `Ok(Some(seq))` path is `length == 17` with a valid `RunSnapshot` decode; all other lengths yield `Err(TrimError::IncompleteTrim { .. })`. Property pressure is the canonical lane for hostile_input + bounded_state per `references/risk-taxonomy.md`. |
| `required_obligation_ids` | `["PO-003-PROPTEST"]` |
| `non_applicability_evidence_refs` | `[]` |
| `limitation_kind` | `""` |

### VLD-004 — `integration / existing planted-malformed-key tests` ⇒ `proptest` verifier

| Field | Value |
|---|---|
| `id` | `VLD-004` |
| `requirement_id` | `REQ-CAP-002`, `REQ-CAP-003`, `REQ-CAP-004`, `REQ-CAP-010` |
| `contract_clause` | `CC-CAP-002`, `CC-CAP-003`, `CC-CAP-004`, `CC-CAP-009`, `CC-CAP-010` |
| `proof_seed_id` | `PS-CAP-UNIT-001`, `PS-CAP-UNIT-002`, `PS-CAP-UNIT-003`, `PS-CAP-REGRESSION-001` |
| `verifier` | `proptest` |
| `risk_tags` | `["parser/codec", "persistence", "hostile_input", "rejection", "bounded_state"]` |
| `applicability` | `required` |
| `decision_reason` | The existing temp_journal-backed integration tests at `snapshot_tests.rs:208-248` (CC-CAP-002) and `trimming/tests.rs:875-987` (CC-CAP-003, CC-CAP-004) plant malformed raw keys via `journal.run_snapshot.insert(...)` / `journal.events.insert(...)`. The proof strategy augments with three overlong-key cases (CC-CAP-010). Routed through `proptest` verifier vocabulary because the schema has no separate `integration` verifier. |
| `required_obligation_ids` | `["PO-002-INTEGRATION"]` |
| `non_applicability_evidence_refs` | `[]` |
| `limitation_kind` | `""` |

### VLD-005 — `moon-source-lint / lint-src + workspace` ⇒ `proptest` verifier

| Field | Value |
|---|---|
| `id` | `VLD-005` |
| `requirement_id` | `REQ-CAP-008`, `REQ-CAP-009` |
| `contract_clause` | `CC-CAP-008`, `CC-CAP-009` |
| `proof_seed_id` | `PS-CAP-CROSS-CRATE-001`, `PS-CAP-REGRESSION-001` |
| `verifier` | `proptest` |
| `risk_tags` | `["public_api", "numeric/cap_refinement", "parse_canonicalization"]` |
| `applicability` | `required` |
| `decision_reason` | CC-CAP-008 requires `moon run :lint-src` (Holzman Rust zero-tolerance source lint) + `cargo check --workspace` to pass with zero changes to `vb_core`, `vb_runtime`, `vb_cli`, `vb_validate`. The literal-replacement invariant (magic `17` → `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` at `trimming/logic.rs:36, 77, 222`) is verified by `rg -n 'key\.len\(\) != 17' crates/vb_storage/src/trimming/logic.rs` returning no matches. Routed through `proptest` verifier vocabulary because the schema has no separate `clippy` or `lint` verifier; the obligation is a property test of the cap-enforcement invariant across the lint+build surface. |
| `required_obligation_ids` | `["PO-004-LINT"]` |
| `non_applicability_evidence_refs` | `[]` |
| `limitation_kind` | `""` |

---

## Not-applicable lane rows

### VLD-006 — `kani` ⇒ `not_applicable` (`surface_absent`)

| Field | Value |
|---|---|
| `id` | `VLD-006` |
| `requirement_id` | `REQ-CAP-001` |
| `contract_clause` | `CC-CAP-001` |
| `proof_seed_id` | `PS-CAP-KANI-OMIT-001` |
| `verifier` | `kani` |
| `risk_tags` | `["numeric/cap_refinement"]` |
| `applicability` | `not_applicable` |
| `decision_reason` | The only non-trivial bound is a `usize` const alias to another `usize` const literal; verifying a const against itself is vacuous (no exec fn, no loop, no arithmetic). The const alias chain is a compile-time invariant enforced by the `const X = Y` syntax and discharged by `cargo check`. |
| `required_obligation_ids` | `[]` |
| `non_applicability_evidence_refs` | `["130ff5b4e9ba61e022ec86e4f6ec55622c2bfc556062002edd8facde396a4d49"]` (proof-seeds.jsonl row 12) |
| `limitation_kind` | `surface_absent` |

### VLD-007 — `verus` ⇒ `not_applicable` (`surface_absent`)

| Field | Value |
|---|---|
| `id` | `VLD-007` |
| `requirement_id` | `REQ-CAP-005` |
| `contract_clause` | `CC-CAP-005` |
| `proof_seed_id` | `PS-CAP-VERUS-OMIT-001` |
| `verifier` | `verus` |
| `risk_tags` | `["public_api"]` |
| `applicability` | `not_applicable` |
| `decision_reason` | No new `exec fn` is introduced; the existing `TrimError::IncompleteTrim { deleted_count: u64 }` variant is preserved verbatim and its `0x4102` code is already covered by `error_code_tests.rs:~246`. The bead is a numeric/cap refinement, not a functional invariant. |
| `required_obligation_ids` | `[]` |
| `non_applicability_evidence_refs` | `["130ff5b4e9ba61e022ec86e4f6ec55622c2bfc556062002edd8facde396a4d49"]` (proof-seeds.jsonl row 13) |
| `limitation_kind` | `surface_absent` |

### VLD-008 — `flux-rs` ⇒ `not_applicable` (`risk_out_of_scope`)

| Field | Value |
|---|---|
| `id` | `VLD-008` |
| `requirement_id` | `REQ-CAP-001` |
| `contract_clause` | `CC-CAP-001` |
| `proof_seed_id` | `PS-CAP-FLUX-OMIT-001` |
| `verifier` | `flux-rs` |
| `risk_tags` | `["numeric/cap_refinement"]` |
| `applicability` | `not_applicable` |
| `decision_reason` | The cap is a `usize` const alias; refining a `usize` value already discharged by `const X = JOURNAL_KEY_BYTES` adds no information. Flux's SMT-decidable fragment is targeted at refinement types (e.g., `Offset stays within [0, LEN)`), not at const equality. The contract (CC-CAP-001) commits to a unit-test proof, not a Flux refinement. |
| `required_obligation_ids` | `[]` |
| `non_applicability_evidence_refs` | `["130ff5b4e9ba61e022ec86e4f6ec55622c2bfc556062002edd8facde396a4d49"]` (proof-seeds.jsonl row 14) |
| `limitation_kind` | `risk_out_of_scope` |

### VLD-009 — `cargo-fuzz` ⇒ `not_applicable` (`surface_absent`)

| Field | Value |
|---|---|
| `id` | `VLD-009` |
| `requirement_id` | `REQ-CAP-001` |
| `contract_clause` | `CC-CAP-001` |
| `proof_seed_id` | `PS-CAP-FUZZ-OMIT-001` |
| `verifier` | `cargo-fuzz` |
| `risk_tags` | `["hostile_input"]` |
| `applicability` | `not_applicable` |
| `decision_reason` | Key encoders (`run_event_key`, `run_snapshot_key`) are pure 1-input=1-output `ArrayVec<u8, JOURNAL_KEY_BYTES>` writes; the return type is fixed-size `[u8; 17]`, so fuzzing the encoder adds no coverage beyond the proptest roundtrip on the length property. The decoders (`decode_storage_key`, `decode_run_event_key`) already enforce length equality via `KeyDecodeError::KeyLengthMismatch`; their hostile-input surface is covered by the proptest length roundtrip (VLD-003) and the integration tests (VLD-004). No parser-with-arbitrary-byte-input is added by this bead. |
| `required_obligation_ids` | `[]` |
| `non_applicability_evidence_refs` | `["130ff5b4e9ba61e022ec86e4f6ec55622c2bfc556062002edd8facde396a4d49"]` (proof-seeds.jsonl row 15) |
| `limitation_kind` | `surface_absent` |

### VLD-010 — `loom` ⇒ `not_applicable` (`surface_absent`)

| Field | Value |
|---|---|
| `id` | `VLD-010` |
| `requirement_id` | `REQ-CAP-006` |
| `contract_clause` | `CC-CAP-006` |
| `proof_seed_id` | `PS-CAP-LOOM-OMIT-001` |
| `verifier` | `loom` |
| `risk_tags` | `["bounded_state"]` |
| `applicability` | `not_applicable` |
| `decision_reason` | The trim scanners (`latest_durable_snapshot_seq`, `trim_events_for_run`, `count_trimmable_events`) are synchronous, single-threaded, and operate on a per-journal Fjall snapshot (`Database::snapshot()` at `trimming/logic.rs:214`). There is no `Send`/`Sync` boundary, no `spawn`, no async, no shared mutable state. Loom's schedule exploration is for interleavings across threads; the trim path has no interleavings to model. |
| `required_obligation_ids` | `[]` |
| `non_applicability_evidence_refs` | `["130ff5b4e9ba61e022ec86e4f6ec55622c2bfc556062002edd8facde396a4d49"]` (proof-seeds.jsonl row 16) |
| `limitation_kind` | `surface_absent` |

---

## Pairing cross-reference

| PO row | VLD rows that cite it |
|---|---|
| PO-001-UNIT | VLD-001 |
| PO-001-REGRESSION | VLD-002 |
| PO-002-INTEGRATION | VLD-004 |
| PO-003-PROPTEST | VLD-003 |
| PO-004-LINT | VLD-005 |

| VLD row | PO rows it requires |
|---|---|
| VLD-001 | PO-001-UNIT |
| VLD-002 | PO-001-REGRESSION |
| VLD-003 | PO-003-PROPTEST |
| VLD-004 | PO-002-INTEGRATION |
| VLD-005 | PO-004-LINT |
| VLD-006 | (none) |
| VLD-007 | (none) |
| VLD-008 | (none) |
| VLD-009 | (none) |
| VLD-010 | (none) |

Bidirectional pairing passes Gate 3 of `references/plan-quality-gates.md`.

END OF VERIFIER LANE MATRIX.