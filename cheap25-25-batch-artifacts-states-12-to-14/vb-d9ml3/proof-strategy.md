# Proof Strategy — vb-d9ml3 (Storage trim/snapshot key length cap, P1)

> Bead ID: `vb-d9ml3` — Storage: reject overlong malformed trim and snapshot keys (P1)
> Planner invocation: `proof-planner-vb-d9ml3-state4`
> Parent state: 3 (rust-contract artifacts delivered: `contract.md`, `proof-seeds.jsonl`, `traceability-matrix.jsonl`, `codebase-map.md`, `delivery-scope.jsonl`)
> Owner state: 4
> Current state target: 4 (planning) → 4b (proof-plan-reviewer) → 5 (proof-writer) → 7 (proof-to-implementation) → 12 (formal-verifier)
> Workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-d9ml3`
> Captured: 2026-07-01 (Go-skill pipeline date)
> Controller: femdation

---

## 1. Bead summary

Storage key parsing for the trim and snapshot keyspaces (`PREFIX_RUN_EVENT = 0x11`,
`PREFIX_RUN_SNAPSHOT = 0x12`) currently rejects non-canonical keys via three
`if key.len() != 17 { ... }` length checks at `crates/vb_storage/src/trimming/logic.rs`
lines 36, 77, 222. The integer literal `17` is a magic number; the bead asks for
named caps (`MAX_TRIM_KEY_LEN`, `MAX_SNAPSHOT_KEY_LEN`) co-located with the
existing `JOURNAL_KEY_BYTES` constant. Typed error: `TrimError::IncompleteTrim
{ deleted_count: u64 }` with diagnostic code `0x4102` is **preserved verbatim** —
the implementation must NOT converge on `JournalError::MalformedKeyspaceRow`
(diagnostic `0x4030`), because that would break the existing structural test
assertions at `crates/vb_storage/src/snapshot_tests.rs:235` and
`crates/vb_storage/src/trimming/tests.rs:929, 984`.

The planner treats this as a **numeric/cap refinement** with three call-site
edits and zero new runtime behavior. All four planned obligations are
`behavior_affecting: false` — the cap is enforcement of an invariant the code
already discharged via the literal `17`, and the new aliases make the contract
explicit without altering acceptance/rejection semantics.

---

## 2. Risk profile

| Risk tag | Where it triggers | Lane profile |
|---|---|---|
| `parser/codec` | `keys.rs` decoder (`decode_storage_key`) + `trimming/logic.rs` scanner | proptest + unit |
| `persistence` | Fjall `put_snapshot` + trim loops reading raw key bytes | integration |
| `public_api` | `TrimError::IncompleteTrim` shape; diagnostic code `0x4102`; new `pub(crate) const` aliases | unit |
| `error_taxonomy` | New `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN` constants and the typed path that consumes them | unit |
| `numeric/cap_refinement` | The `usize` const alias chain (`MAX_TRIM_KEY_LEN = JOURNAL_KEY_BYTES`) | proptest |
| `bounded_state` | Length-bounded decoder surface (`0..=1024` raw keys) | proptest |
| `hostile_input` | Raw key planting in integration tests + proptest length roundtrip | proptest + integration |
| `concurrency` | NOT in scope (trim scanners are synchronous, single-threaded) | not_required |
| `temporal` | NOT in scope (no recovery from wrong snapshot) | not_required |
| `unsafe/UB` | NOT in scope (`vb_storage` is `#![forbid(unsafe_code)]`) | not_required |

The risk class for the schema-level `proof-obligation/v1.risk` field is split:

- PO-001 (const-equality): `risk: equality`
- PO-002 (integration overlong): `risk: rejection`
- PO-003 (length roundtrip): `risk: rejection`
- PO-004 (lint + workspace): `risk: parse_canonicalization`

---

## 3. Lane profile (canonical, per contract.md §"Verifier Lane Profile" + delivery-scope.jsonl rows 32–39)

| Lane | Status | Rationale |
|---|---|---|
| `default_rust_lane` (unit) | **REQUIRED** | CC-CAP-001 const-equality + CC-CAP-005 variant preservation; maps to schema verifier `proptest` (no schema-level `unit` verifier exists; cargo test is exercised through the proptest verifier vocabulary per skill SKILL.md anti-discipline). |
| `proptest` (length roundtrip) | **REQUIRED** | CC-CAP-001/002/003/004: arbitrary-length key generator; one proptest module exercises encoder-side length invariant and decoder-side rejection invariant. |
| `integration` (existing planted-malformed-key tests) | **REQUIRED** | Existing temp_journal-backed tests at `snapshot_tests.rs:208-248` and `trimming/tests.rs:875-987` plus three new overlong cases (CC-CAP-010). |
| `moon-source-lint` (lint-src + workspace check) | **REQUIRED** | CC-CAP-008: zero cross-crate change; CC-CAP-009: existing tests continue to pass; literal-replacement invariant (magic `17` → `MAX_TRIM_KEY_LEN` / `MAX_SNAPSHOT_KEY_LEN`) at `trimming/logic.rs:36, 77, 222`. |
| `kani` | **NOT_APPLICABLE** | Pure numeric/cap refinement against an already-bounded `JOURNAL_KEY_BYTES = 17`; const alias chain is compile-time. Seed PS-CAP-KANI-OMIT-001. |
| `verus` | **NOT_APPLICABLE** | No new `exec fn` with a non-trivial bound; existing `TrimError::IncompleteTrim` is preserved verbatim. Seed PS-CAP-VERUS-OMIT-001. |
| `flux` | **NOT_APPLICABLE** | The cap is a `usize` const alias; refining a `usize` value already discharged by const adds no information. Seed PS-CAP-FLUX-OMIT-001. |
| `fuzz` | **NOT_APPLICABLE** | Encoders are pure 1-input=1-output `ArrayVec` writes; fuzzing adds no coverage beyond proptest roundtrip on the decoder. Seed PS-CAP-FUZZ-OMIT-001. |
| `looom` (loom) | **NOT_APPLICABLE** | No concurrent state-transition surface; trim scanners are synchronous. Seed PS-CAP-LOOM-OMIT-001. |

The five omitted lanes (`kani`, `verus`, `flux`, `fuzz`, `loom`) each carry a
dedicated `verifier-lane-decision/v1` row with `applicability: not_applicable`,
typed `limitation_kind` (`surface_absent` for kani/verus/fuzz; `risk_out_of_scope`
for flux; `surface_absent` for loom), and concrete `non_applicability_evidence_refs`
pointing at the SHA-256 hashes of the proof-seed rows that document each
omission. No waivers are required (the obligations are non-behavior; see §6).

---

## 4. Source-of-truth artefacts (referenced by every obligation)

| Artifact | Path | SHA-256 |
|---|---|---|
| Codebase map | `.beads/vb-d9ml3/codebase-map.md` | `e813015767c859f3290ad0e5c6edbea8f1f75d0643290f74fa870122737adcfe` |
| Contract | `.beads/vb-d9ml3/contract.md` | `fe425266234443d6ab26056e1bc2b090f730b94b05b6bae378174813b070a8f9` |
| Proof seeds | `.beads/vb-d9ml3/proof-seeds.jsonl` | `130ff5b4e9ba61e022ec86e4f6ec55622c2bfc556062002edd8facde396a4d49` |
| Delivery scope | `.beads/vb-d9ml3/delivery-scope.jsonl` | `596db8f407c6bfb4b7dec3cbbe7cf0eb2bca89d85912412bc2fe72162fbbf691` |
| Traceability matrix | `.beads/vb-d9ml3/traceability-matrix.jsonl` | `13e2054bbeda152c43edfb1f7acb032a9822718c91188c2027d97af32bde875a` |

Production source symbols under test (each `proof-obligation/v1.target` MUST
parse as `path::symbol`):

- `crates/vb_storage/src/constants.rs::MAX_TRIM_KEY_LEN`
- `crates/vb_storage/src/constants.rs::MAX_SNAPSHOT_KEY_LEN`
- `crates/vb_storage/src/constants.rs::JOURNAL_KEY_BYTES`
- `crates/vb_storage/src/trimming/logic.rs::latest_durable_snapshot_seq` (line 26)
- `crates/vb_storage/src/trimming/logic.rs::trim_events_for_run` (line 49)
- `crates/vb_storage/src/trimming/logic.rs::count_trimmable_events` (line 208)
- `crates/vb_storage/src/trimming/mod.rs::TrimError::IncompleteTrim` (line 51)
- `crates/vb_storage/src/trimming/mod.rs::TrimError::INCOMPLETE_TRIM_CODE` (line 62)
- `crates/vb_storage/src/trimming/mod.rs::TrimError::diagnostic_code` (line 65)
- `crates/vb_storage/src/keys.rs::run_event_key` (line 81)
- `crates/vb_storage/src/keys.rs::run_snapshot_key` (line 86)
- `crates/vb_storage/src/error/mod.rs::JournalError::Trim` (line 187)
- `crates/vb_storage/src/error/codes.rs::MALFORMED_KEYSPACE_ROW_CODE` (line 95) — referenced for the **forbidden** path; the planner MUST NOT route through this code.

---

## 5. Proof strategy by requirement

### 5.1 REQ-CAP-001 — const-alias equality (CC-CAP-001)

The const-alias chain `MAX_TRIM_KEY_LEN = JOURNAL_KEY_BYTES`,
`MAX_SNAPSHOT_KEY_LEN = JOURNAL_KEY_BYTES` is a compile-time invariant. The
proof strategy is a single `cargo test -p vb_storage --lib` invocation of one
new unit test (`max_key_len_aliases_equal_journal_key_bytes`) that asserts both
aliases equal `JOURNAL_KEY_BYTES`. Because the alias chain uses `const X = Y`
(not literal `17`), any future drift at `JOURNAL_KEY_BYTES` propagates to both
aliases and the unit test fails. Behaviour-affecting: false (the alias chain
has the same value as the literal `17` it replaces).

### 5.2 REQ-CAP-002 — `latest_durable_snapshot_seq` rejects overlong snapshot key (CC-CAP-002, CC-CAP-010)

The existing regression at `snapshot_tests.rs:208-248` plants a 13-byte key
under `PREFIX_RUN_SNAPSHOT` and asserts `Err(TrimError::IncompleteTrim {
deleted_count: 0 })`. The proof strategy augments this with a new test
(`latest_durable_snapshot_seq_rejects_overlong_snapshot_key`, co-located at
~line 248) that plants a 24-byte key (9 prefix+run bytes + 15 trailing bytes)
under the same prefix and asserts the same `Err(TrimError::IncompleteTrim
{ deleted_count: 0 })`. Behaviour-affecting: false (the underlying rejection
semantics are unchanged; the test only exercises the literal cap at the
opposite end of the size spectrum).

### 5.3 REQ-CAP-003 — `trim_events_for_run` rejects overlong event key (CC-CAP-003, CC-CAP-010)

Existing test at `trimming/tests.rs:875-932` plants a 9-byte key under
`PREFIX_RUN_EVENT` and asserts `Err(TrimError::IncompleteTrim { .. })`. The
proof strategy augments with a new test
(`trim_events_for_run_fails_closed_on_overlong_event_key`, co-located at
~line 932) that plants a 24-byte key under the same prefix after a valid 17-byte
event and asserts the same error. Behaviour-affecting: false.

### 5.4 REQ-CAP-004 — `count_trimmable_events` rejects overlong event key (CC-CAP-004, CC-CAP-010)

Existing test at `trimming/tests.rs:934-987` wraps the rejection through
`JournalError::Trim(IncompleteTrim { .. })`. The proof strategy augments with
a new test (`trim_eligibility_diagnostic_fails_closed_on_overlong_event_key`,
co-located at ~line 987) that plants a 24-byte key and asserts the same wrapped
error. Behaviour-affecting: false.

### 5.5 REQ-CAP-005 — `TrimError::IncompleteTrim` shape and 0x4102 preservation (CC-CAP-005)

The existing test at `error_code_tests.rs:~246`
(`journal_error_trim_wrapper_delegates_incomplete_trim_code`) asserts the
`0x4102` propagation through `JournalError::Trim`. The proof strategy pins
this test as the regression gate; no new test is required. Behaviour-affecting:
false (the variant shape and code are unchanged).

### 5.6 REQ-CAP-006 — fail-closed workflow (CC-CAP-006)

The three existing integration tests already plant non-canonical keys BEFORE
valid keys and assert the abort. The proof strategy relies on the existing
tests + the three new overlong cases (CC-CAP-010). Behaviour-affecting: false.

### 5.7 REQ-CAP-007 — counter progress preservation (CC-CAP-007)

The `deleted_count` field preserves partial progress. The existing
`trimming/tests.rs:929, 984` tests use `IncompleteTrim { .. }` (matches any
counter value); the planner does not introduce a stronger pin (the contract
permits any counter value). Behaviour-affecting: false.

### 5.8 REQ-CAP-008 — zero cross-crate change (CC-CAP-008)

The proof strategy is `moon run :lint-src` + `cargo check --workspace`. Both
must pass post-fix with no diff in `vb_core`, `vb_runtime`, `vb_cli`,
`vb_validate`. The lint command (`lint-src`) is the canonical zero-tolerance
source lint (Holzman Rust); cargo check verifies the build graph. Behaviour-
affecting: false (no cross-crate API surface is touched).

### 5.9 REQ-CAP-009 — existing tests continue to pass (CC-CAP-009)

The proof strategy is the existing `cargo test -p vb_storage --lib
snapshot_tests` and `cargo test -p vb_storage --lib trimming::tests`
invocations. Both must be GREEN post-fix without modification of assertion
structure. Behaviour-affecting: false.

### 5.10 REQ-CAP-010 — three new overlong test cases (CC-CAP-010)

The proof strategy is the augmentation described in §5.2–§5.4 (one new test
per magic-17 call site). Behaviour-affecting: false.

---

## 6. Behavior-affecting discipline

The planner overrides the seed-level `behavior_affecting: true` flags on
PS-CAP-UNIT-001/002/003, PS-CAP-PROPTEST-001/002, and PS-CAP-WORKFLOW-001 to
`false` for the following reasons:

1. The change is **enforcement of an invariant the code already discharged**.
   The existing `key.len() != 17` check rejected any non-canonical key
   (length < 17 OR length > 17 OR length != 17). The new aliases
   `MAX_TRIM_KEY_LEN = JOURNAL_KEY_BYTES = 17` and `MAX_SNAPSHOT_KEY_LEN =
   JOURNAL_KEY_BYTES = 17` are **named caps on the same number**; they do not
   introduce new acceptance/rejection conditions.
2. The typed error variant (`TrimError::IncompleteTrim { deleted_count: u64 }`)
   and its diagnostic code (`0x4102`) are **preserved verbatim** (CC-CAP-005).
3. The overlong-key test cases (CC-CAP-010) **pass against the pre-fix code
   too** — the existing `key.len() != 17` check at lines 36, 77, 222 already
   rejects overlong keys. The new tests pin this behavior but do not introduce
   new behavior.
4. The bead text calls this a **P1 bug closure** (Round 10 issue 7), not a
   behavior change. The fix is a *named-cap refactor*, not a semantic change.

Because all four obligations are `behavior_affecting: false`:

- `proof-to-implementation-input.md` is **out of scope** for this bead (the
  bridge is only required when behavior-affecting obligations exist).
- `waiver-candidates.jsonl` is **empty** (the proof-seed omissions for
  kani/verus/flux/fuzz/loom are recorded as `verifier-lane-decisions.jsonl`
  `applicability: not_applicable` rows, not as waiver candidates).
- `trusted-base-plan.md` records only the const-alias chain as a single
  compile-time trust marker; no Miri specialist scoping note is needed (no
  `unsafe` risk_tags).

---

## 7. Constraint propagation

The following constraints from `contract.md` and the task directive propagate
to every obligation:

- **Forbidden**: defining `MAX_TRIM_KEY_LEN` or `MAX_SNAPSHOT_KEY_LEN` as a
  literal `17` at the alias site (the alias MUST be `const X = JOURNAL_KEY_BYTES`).
- **Forbidden**: introducing a new `TrimError` variant for overlong keys.
- **Forbidden**: converging on `JournalError::MalformedKeyspaceRow` for the
  trim path (diagnostic code `0x4030`, different shape; would break
  `snapshot_tests.rs:235` and `trimming/tests.rs:929, 984`).
- **Forbidden**: adding a new diagnostic code (`0x4104`, etc.).
- **Forbidden**: silently truncating or padding the raw key (must fail closed).
- **Required**: every existing structural assertion (`Err(TrimError::IncompleteTrim
  { deleted_count: 0 })` at `snapshot_tests.rs:235`; `Err(TrimError::IncompleteTrim { .. })`
  at `trimming/tests.rs:929, 984`) continues to pass unmodified.

---

## 8. Handoff

- `proof-plan-reviewer` at State 4b dispositions each lane decision and
  obligation; this planner's `verifier-lane-decisions.jsonl` and
  `proof-obligations.planned.jsonl` are the reviewer input.
- `proof-writer` at State 5 authors the unit test (PO-001), three overlong-key
  integration tests (PO-002), the proptest module (PO-003), and any lint-config
  adjustments (PO-004). The proof-writer does NOT edit production Rust (the
  implementation agent at State 6 owns the magic-17 replacement at
  `trimming/logic.rs:36, 77, 222` and the alias declaration at
  `constants.rs:74-79`).
- `formal-verifier` at State 12 executes each obligation's `command` and
  records raw evidence in `verification-ledger/v1`.

---

## 9. Self-audit against `references/plan-quality-gates.md`

| Gate | Status |
|---|---|
| Gate 1 — Schema compliance | Planned (validator not run; field shape reviewed against schema). |
| Gate 2 — Lane decision coverage | Planned: 9 VLD rows (4 required + 5 not_applicable); one per (req, cc, seed, verifier) tuple in scope. |
| Gate 3 — Obligation pairing | Planned: each required VLD row has ≥1 paired PO row; each PO row's `target` is a production source symbol. |
| Gate 4 — Implementation binding | Planned: every `target` parses as `path::symbol`; no Verus obligation so no `external_body`/`assume`/`axiom` risk. |
| Gate 5 — Evidence specificity | Planned: every `command` is exact; every `workdir` is absolute; every `expected_evidence` cites a concrete tool marker. |
| Gate 6 — Resource governance | Planned: every `verifier: proptest` obligation includes `PROPTEST_CASES` and `model_bounds.cases` / `model_bounds.input_size`. |
| Gate 7 — Waiver discipline | Planned: `waiver-candidates.jsonl` is empty (no waivers needed; lane omissions are VLD `not_applicable` rows). |
| Gate 8 — Trust marker ledger | Planned: one `TB-CAP-001` row in `trusted-base-plan.md` for the const alias chain. |
| Gate 9 — Cross-reference integrity | Planned: every `behavior_affecting: false` PO row; no `rust-refinement-obligation/v1` rows required. |
| Gate 10 — Mirror parity | Out of scope for this bead (the skill tree is at `~/.agents/skills/proof-planner/` and `~/.opencode/skill/proof-planner/`; this planner does not modify those trees). |

END OF PROOF STRATEGY.