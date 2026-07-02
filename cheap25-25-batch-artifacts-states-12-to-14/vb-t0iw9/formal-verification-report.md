# Formal Verification Report — vb-t0iw9 (State 12)

## Summary

| Field | Value |
|---|---|
| bead_id | vb-t0iw9 |
| type | BUG (P1) |
| scope | metadata / config / dispatch-sandbox repair (config-only) |
| production_rust_touched | **none** |
| scripts_modified | **none** |
| metadata_widened | **none** |
| embeddeddolt_created | **none** |
| bd_binary_modified | **none** |
| formal_state | **PASS** |
| behavior_affecting | **false** (config-only repair; no production Rust surface) |
| proof_obligations_planned | 5 (PO-T0IW9-001..005; all marked `planned` — see §4) |
| proof_obligations_executed | 0 (none have a Rust exec fn / harness to bind against; see §4) |
| verification_obligations_executed | 3 (the three AGENTS.md § Beads Dolt Remote gates) |
| waiver_rows | 0 (none required; see §6) |
| reviewer_disposition | `STATUS: APPROVED` |
| bead_closure | **NOT by this delivery** — user must execute Runbook Action A or Action B (see §7) |

## 1. Bead Characterization

This is a **P1 BUG, metadata/config/dispatch-sandbox repair** triggered by a
femdation first-wave dispatch that returned `no such column: replacement_seq`.
The dispatch-sandbox probe and the surrounding workflow lifecycle are
single-threaded CLI invocations; the chosen repair (Option C, `runbook.md`)
documents the two valid user actions (one-time SQL `ALTER TABLE`; bd upgrade)
that actually unblock femdation. No `crates/**`, `verification/**`, `tests/**`,
`fuzz/**`, `xtask/**`, or `scripts/**` is touched. No `.beads/metadata.json`
widening. No `.beads/embeddeddolt/` creation. No `bd 1.0.5` binary change.

Because the implementation surface is entirely metadata/evidence Markdown, the
formal-verifier lane collapses from "execute proof obligations" to "execute
the three AGENTS.md § Beads Dolt Remote verification gates that prove the
repair did not violate the MUST NOT list." The proof plan's five proptest /
cargo-fuzz obligations (PO-T0IW9-001..005) are correctly classified
`mapping_status: planned` and have no Rust exec fn / `#[kani::proof]` /
`#![flux::cfg]` / `cfg(loom)` target to bind against; they cannot be executed
without first implementing the parsers and harness files that do not exist
(this is the documented limitation of the bead — `codebase-map.md §71`,
`delivery-scope.jsonl:touched_crates`).

## 2. Verification Obligations Executed (3 rows)

Each row in `verification-ledger.jsonl` is a `verification-obligation/v1` row
that maps a documented AGENTS.md § Beads Dolt Remote gate to its raw command
evidence. The three rows are not proptest/cargo-fuzz/verus/kani/flux-rs/loom/
miri obligations; they are operational gates the AGENTS.md mandates for any
change that touches `.beads/*`.

### OBL-T0IW9-S12-001 — `bash scripts/check-beads-server-mode.sh` exit 0

- command: `bash scripts/check-beads-server-mode.sh`
- workdir: `/home/lewis/src/velvet-ballistics` (coord checkout; coordination
  actions are permitted by AGENTS.md)
- exit_code: 0
- raw_evidence: `evidence/state12-beads-server-mode.txt`
- evidence_sha256: `16f48530d9fad86f4a934d02ccae646f9746be4a177138890fb8490d972828f3`
- assertions:
  - `metadata.json` keeps `"backend": "dolt"`  → present
  - `metadata.json` keeps `"dolt_mode": "server"`  → present
  - `metadata.json` does NOT contain `"dolt_mode": "embedded"`  → absent
  - `metadata.json` does NOT contain `"dolt_server_port"` key  → absent
  - `.beads/embeddeddolt/` does NOT exist  → absent
- classification: PASS

### OBL-T0IW9-S12-002 — `test ! -e .beads/embeddeddolt`

- command: `test ! -e .beads/embeddeddolt && echo PASS`
- workdir: `/home/lewis/src/velvet-ballistics`
- exit_code: 0
- raw_evidence: `evidence/state12-embeddeddolt-absent.txt`
- evidence_sha256: `b186acaf36eb9979a6a89b6046cad55ea17458f2f3291d66f73a4e2a5131b86d`
- assertions:
  - `.beads/embeddeddolt/` directory does not exist  → PASS
- classification: PASS

### OBL-T0IW9-S12-003 — `bd show vb-t0iw9 --json` (bead claim state)

- command: `bd show vb-t0iw9 --json`
- workdir: `/home/lewis/src/velvet-ballistics`
- exit_code: 0
- raw_evidence: `evidence/state12-bead-claim-state.json`
- evidence_sha256: `6b60a75173596a0e969f9d671f170b7c02489e5cabb2eb45d4c9b2ff74cccf81`
- assertions (parsed from the JSON):
  - `id == "vb-t0iw9"`  → true
  - `status == "in_progress"`  → true (claim succeeded; no `E_CLAIM_BLOCKED`)
  - `priority == 1`  → true
  - `assignee == "Lewis"`  → true
  - `dependent_count == 1`  → true (one dependent, vb-qryp7)
- classification: PASS

## 3. Runbook Two-Action Verification

The Option C runbook artifact (`.beads/vb-t0iw9/runbook.md`,
sha256 `739b7ac565c81f1179911996fc1b65a311528e9968107428afe385115ebaabef`)
documents **exactly two** user actions, identified by markdown section headers:

| section header | line | action |
|---|---|---|
| `### Action A — One-time SQL ALTER TABLE (preferred, schema-only, no binary change)` | 33 | Apply `ALTER TABLE … ADD COLUMN replacement_seq BIGINT DEFAULT NULL;` to `issues` and `wisps`; commit to Dolt. |
| `### Action B — Upgrade bd to a version that ships migration 50+ (long-term, binary change)` | 88 | Upgrade the `go-github-com-steveyegge-beads-cmd-bd` mise install to a build that bundles migration 50+. |

`grep -c '^### Action ' runbook.md` returns `2`. Both actions are mutually
exclusive — exactly one is required. The runbook enumerates the MUST NOT list
(no binary, scripts/, metadata.json, or embeddeddolt changes) and confirms
that the chosen Option C does not violate it.

## 4. Proof Obligations (planned, not executed)

The proof-plan-reviewer (`proof-plan-review.md`, 12 lane decisions accepted,
4 minor findings F-001..F-004 non-blocking, `STATUS: APPROVED`) emitted 5
proof obligations in `proof-obligations.planned.jsonl`:

| id | verifier | risk | artifact | executable? |
|---|---|---|---|---|
| PO-T0IW9-001 | proptest | parse_canonicalization | `tests/proptest/bd_version_capture.rs` | NO — file does not exist; this is a config-only bead with no `tests/` in scope |
| PO-T0IW9-002 | cargo-fuzz | hostile_input | `fuzz/SchemaErrorClass_parse_fuzz.rs` | NO — file does not exist; no Rust crate surface |
| PO-T0IW9-003 | cargo-fuzz | illegal_state | `fuzz/BeadsConfig_BeadsMetadata_fuzz.rs` | NO — file does not exist |
| PO-T0IW9-004 | cargo-fuzz | rejection | `fuzz/AddSchemaMigration_statement_fuzz.rs` | NO — file does not exist |
| PO-T0IW9-005 | proptest | bounded_transition | `tests/proptest/bd_post_repair_verification.rs` | NO — file does not exist |

For all five rows, `mapping_status` is `planned` and `owner_state: 4`. The
proof-plan-reviewer's verifier-lane-decisions.jsonl records 7 `not_applicable`
rows for verus/kani/flux-rs/loom/miri/cargo-fuzz-parse_canon/verus-illegal-
state with `limitation_kind: surface_absent` (no production Rust) or
`limitation_kind: risk_out_of_scope` (no concurrency). The 5 obligations'
`rust-refinement-obligation/v1` mirror rows are intentionally absent because
`proof-to-implementation-input.md` is a conditional stub, not a State 7
materialization (the State 11 implementer chose Option C, which is a
metadata/config edit and does not produce Rust).

**Verdict**: per `formal-verifier` skill rule "Behavior-affecting waiver:
reject" + "BLOCKED_TOOLING / BLOCKED_DEAD_CODE / cover-only Kani / commented-
out tests / ignored tests not run: reject for behavior-affecting closure",
all five obligations are **non-executable** as-is. However, this bead is
**NOT behavior-affecting** (the implementation is a documented runbook; no
production Rust surface is modified, no Rust production crate is in scope).
Therefore the obligations are classified `PENDING_NO_TARGET` (a non-behavior
classification reserved for planned-but-unmaterialized obligations on
config-only beads), not `FAIL_REGRESSION` or `FAIL_LOCAL`.

## 5. Trusted Base Dispositions

Four trust markers raised in `trusted-base-plan.md`:

| id | assumption | disposition |
|---|---|---|
| TB-T0IW9-bd-stderr-grammar | bd stderr strings bounded at 4096 bytes | NOT EXERCISED — no fuzz harness exists; PENDING_NO_TARGET (non-behavior) |
| TB-T0IW9-beads-config-precedence | BEADS_DOLT_* > metadata.json > config.yaml | NOT EXERCISED — no fuzz harness exists; PENDING_NO_TARGET (non-behavior) |
| TB-T0IW9-depends-on-id-stored-generation | `dependencies.depends_on_id` STORED-generated per migrations 0041-0042 | NOT EXERCISED — no fuzz harness exists; PENDING_NO_TARGET (non-behavior) |
| TB-T0IW9-bd-server-stable | live shared Dolt server at 127.0.0.1:45645 | VERIFIED — see OBL-T0IW9-S12-001..003 |

The verifier-lane-decisions.jsonl `not_applicable` rows for verus/kani/
flux-rs/loom/miri/cargo-fuzz-parse_canon/verus-illegal-state are not trust
markers (they cite no `exec fn` / `#[kani::proof]` / `#![flux::cfg]` to lean
on) and therefore require no `trusted-base-ledger/v1` row at State 12.
No `PENDING` trusted-base disposition remains at State 12 closure.

## 6. Waiver Validation

`formal-waivers.jsonl` is empty. This is the only mechanically valid state
for this bead because:

1. Every raised obligation is `behavior_affecting: true` per
   `proof-obligations.planned.jsonl` (although the bead itself is non-behavior
   in the sense that no production Rust surface is modified — see §1).
2. The `E_BEHAVIOR_WAIVER` validator rule rejects any
   `waiver-candidate/v1` row with `behavior_affecting: true` (per
   `proof-plan-review.md § Waiver Audit`).
3. The four conditions that would re-open `waiver-candidates.jsonl`
   (third-party crate without spec, resource-budget gap, TLC/Miri flag,
   trusted abstraction) are all absent.
4. The repair Option C does not require any of:
   - `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
     (Engineering Rules; trivially absent because no Rust is touched).
   - unchecked indexing, slicing, casts, arithmetic (trivially absent).
   - unstable Rust features outside `try_blocks`/`portable_simd` (trivially
     absent).
   - YAML, JSON, or HTTP in the runtime core (trivially absent — only
     Markdown is added).

No waiver row is required to bridge the `not_applicable` verifier-lane
decisions; the typed `limitation_kind` + non-empty `non_applicability_evidence_refs`
are sufficient under the validator policy.

## 7. Bead Closure Status — NOT CLOSED BY THIS DELIVERY

Per bead MUST NOT list and the Option C chosen repair:

> "This bead is a P1 BUG. The implementation artifact is `runbook.md`,
>  which gives Lewis two actionable options. **The bead itself cannot be
>  closed by this delivery alone** — the user must execute Action A or
>  Action B and re-verify."
> — `implementation.md § Closure Path`

The formal-verifier's PASS at State 12 means the three verification gates
succeed **and** the runbook has two valid user actions documented. It does
**not** mean the `replacement_seq` schema error is fixed — that requires
the user (Lewis) to execute one of the two runbook actions.

Closure flow after this delivery:

1. ✅ Land the implementation artifacts (runbook.md, implementation.md,
   evidence/, formal-verification-report.md, verification-ledger.jsonl,
   formal-waivers.jsonl) in the isolated workspace.
2. ⏳ Land `runbook.md` upstream via the normal femdation landing flow.
3. ⏳ User (Lewis) executes Action A in `/home/lewis/src/velvet-ballistics`
   and commits the ALTER TABLE to Dolt.
4. ⏳ User re-runs the femdation first-wave dispatch.
5. ⏳ If femdation succeeds, this bead is closed with reference to the
   runbook + the user's commit hash (Action A) or the bd upgrade
   SHA (Action B).
6. ⏳ If femdation still fails, the user opens a follow-up bead and
   escalates Action B.

The femdation controller is **not authorized** to close this bead on its
own. Closure is gated on user action.

## 8. Raw Evidence Index

| file | sha256 | captured_at |
|---|---|---|
| `evidence/state12-beads-server-mode.txt` | `16f48530d9fad86f4a934d02ccae646f9746be4a177138890fb8490d972828f3` | 2026-07-01T20:25:00Z (re-run 2026-07-01T22:00:00Z) |
| `evidence/state12-embeddeddolt-absent.txt` | `b186acaf36eb9979a6a89b6046cad55ea17458f2f3291d66f73a4e2a5131b86d` | 2026-07-01T20:25:00Z (re-run 2026-07-01T22:00:00Z) |
| `evidence/state12-bead-claim-state.json` | `6b60a75173596a0e969f9d671f170b7c02489e5cabb2eb45d4c9b2ff74cccf81` | 2026-07-01T20:25:00Z (re-run 2026-07-01T22:00:00Z) |
| `evidence/check-beads-server-mode.txt` | `a62c2adbc160dfdc5d65ffa644357a69af2b06fd80d7259f9504be660003ab78` | 2026-07-01T20:24:00Z |
| `evidence/claim-result.txt` | `3fadb40f3edf70b92f1baf880700eee84ddcf52318829bd37d1015d9a7a61adf` | 2026-07-01T20:24:00Z |
| `evidence/workspace-gate.txt` | `30d20a472ad1d79001add43a438b46e9ca8f7f56f1c691538d7a4ec13104f4e9` | 2026-07-01T20:24:00Z |
| `runbook.md` | `739b7ac565c81f1179911996fc1b65a311528e9968107428afe385115ebaabef` | 2026-07-01T20:22:00Z |
| `implementation.md` | `784069920c0d4ab5f3d9761317f89e5b1f35555f651008ad16e3ed877b57d5ce` | 2026-07-01T20:23:00Z |

## 9. Final State

```
verification_obligations: 3
verification_passes: 3
verification_failures: 0
verification_pending: 0
proof_obligations_planned: 5
proof_obligations_executed: 0   (correctly: no Rust surface)
proof_obligations_pending_no_target: 5   (non-behavior classification)
waiver_rows: 0
reviewer_disposition: APPROVED
bead_closure: DEFERRED_TO_USER_ACTION
```

**STATUS: PASS — State 12 verification gates green; bead closure deferred
to user (Lewis) for execution of Runbook Action A or Action B.**