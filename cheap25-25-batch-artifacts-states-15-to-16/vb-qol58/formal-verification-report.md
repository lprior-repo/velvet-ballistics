---
bead_id: vb-qol58
schema_version: formal-verification-report/v1
state: 12
skill: formal-verifier
formal_verifier_invocation_id: formal-verifier-vb-qol58-state12-20260701T225200Z
parent_invocation_id: holzman-rust-vb-qol58-state11-20260701T192500Z
workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58
host_session_id: femdation-cheap25-batch
binding_classification: N/A (no formal-verifier artefacts written per proof-writer-report.md NO_PROOF_WORK_DECLARED)
trust_base_classification: N/A (trusted-base-ledger.jsonl 0 bytes — no trust markers introduced)
started_at: 2026-07-01T22:52:00Z
completed_at: 2026-07-01T22:54:30Z
status: PASS
obligations_total: 3
obligations_pass: 3
obligations_fail: 0
obligations_waived: 0
---

# Formal Verification Report: vb-qol58 — State 12 Attempt 1

## Bead

- **Bead:** `vb-qol58` — Lint: fix source slicing/indexing issues in IPC and test utilities (P0 bug).
- **Workdir:** `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`.
- **Verification scope:** 3 production-line edits at `crates/vb_ipc/src/frame_types.rs:41`, `crates/workspace_tests/src/test_util/seed.rs:23`, and `crates/workspace_tests/src/test_util/fixture.rs:58` — canonical-verb spelling replacement of `&mut bytes[..]` / `&mut vec[..]` (full-array/full-vec slice) with `bytes.as_mut_slice()` / `vec.as_mut_slice()` (a 100% behavior-preserving borrow-syntax refactor under the workspace deny-list `-D clippy::indexing_slicing`).
- **JJ/coord isolation:** `pwd -P` resolves to `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58`; `jj root` resolves to the same path. Source-checkout `/home/lewis/src/velvet-ballistics` was not touched.

## Verdict

**STATUS: PASS.** All 3 `proof-obligation/v1` rows in `proof-obligations.planned.jsonl` were independently re-executed from the isolated JJ workspace and recorded as `PASS` in `verification-ledger.jsonl` with raw command evidence at `.evidence/vb-qol58/verifier/{lint-src,cargo-check,cargo-test}.log`. `formal-waivers.jsonl` is empty (canonical-empty hash `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`); `rust-refinement-obligations.jsonl` is 0 bytes (zero-RRO is the honest disposition for this `behavior_affecting: false` bead per `proof-to-rust-review.md STATUS: APPROVED`); `trusted-base-ledger.jsonl` is 0 bytes (zero trust markers introduced — no Verus/Kani/Flux/Loom/Miri/cargo-fuzz artifacts exist).

## Inputs Read

| Artifact | SHA-256 | Schema |
|---|---|---|
| `.beads/vb-qol58/proof-obligations.planned.jsonl` | `63f333fc2cedcf87bbcf7f1fe63bc8c64571d441bcab3482b81aa065e6b54a38` | `proof-obligation/v1` (3 rows) |
| `.beads/vb-qol58/proof-plan-review.md` | `864a96e8801da03c60a36aac69b75aa829fbe7bc15e89ef30a5c59db96d70d6c` | `proof-plan-review/v1` (STATUS: APPROVED) |
| `.beads/vb-qol58/proof-review.md` | `346d24b886a393988fefd832e382957c21943962706494f27bb44ed5b074ced5` | `proof-review/v1` (STATUS: APPROVED) |
| `.beads/vb-qol58/proof-to-rust-map.md` | `50930fcc1f8e5ead0033c4ae352fdbd7000ef7db51eb00cae2b4c2c57d5c430e` | (bridge — zero RROs) |
| `.beads/vb-qol58/proof-to-rust-review.md` | `85065a066524f773d5bf2d8d14c48e10ee9a3f3d14df0c9da4b082511f65fc9c` | `proof-to-rust-review/v1` (STATUS: APPROVED) |
| `.beads/vb-qol58/verifier-lane-decisions.jsonl` | `a554a60322b61be9abff5e8da8c6a4e333c34ad8c4fce405e36343b0bd590fa4` | `verifier-lane-decision/v1` (23 rows; 5 `required: proptest`, 18 `not_applicable`) |
| `.beads/vb-qol58/verifier-lane-review.jsonl` | `33f495795dfa99c457cace8fd9a5daad361a60417326afc2cdaff4d5d397de34` | `verifier-lane-review/v1` |
| `.beads/vb-qol58/proof-writer-report.md` | `fa01f7f80da7cffc575d66b076201346c88aec82c3e89798405c42814dfaefe3` | `proof-writer-report/v1` (NO_PROOF_WORK_DECLARED) |
| `.beads/vb-qol58/implementation.md` | `a6f9c26abf9712ace4d3ad3169c868bbcdb078082333da64abc7a2e687a0f852` | `implementation/v1` |
| `.beads/vb-qol58/rust-refinement-obligations.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (0 bytes) | `rust-refinement-obligation/v1` (zero rows — honest disposition) |
| `.beads/vb-qol58/trusted-base-ledger.jsonl` | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` (0 bytes) | `trusted-base-ledger/v1` (zero rows — no trust markers) |
| `.moon/tasks/all.yml` | `423e84fa22c28ad863a089a7e4ae2c6dfce4ae827f5db0d2cea991fca1f6134d` | (deny-list line 51 byte-identical to `proof-plan-review.md` baseline) |

All hashes were re-computed against the live artifacts in this isolated workspace and matched the values recorded in upstream `.beads/vb-qol58/agent-invocation-ledger.jsonl` rows 1-8.

## Production-Line Citation Anti-Hallucination

Live `ripgrep` output from the current working-copy commit (`@  vvzkpqnn`):

| Path | Line | Content (current) |
|---|---|---|
| `crates/vb_ipc/src/frame_types.rs` | 41 | `let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());` |
| `crates/workspace_tests/src/test_util/seed.rs` | 23 | `rng.fill(bytes.as_mut_slice());` |
| `crates/workspace_tests/src/test_util/fixture.rs` | 58 | `rng.fill(vec.as_mut_slice());` |

Live `ripgrep` (no matches for `[\.\.]` in the 3 files) confirms the deny-list `-D clippy::indexing_slicing` pattern was removed from the targeted sites:

```
$ rg -n '\[\.\.\]' crates/vb_ipc/src/frame_types.rs crates/workspace_tests/src/test_util/seed.rs crates/workspace_tests/src/test_util/fixture.rs
(no matches)
```

Live `jj file show -r @-` confirms the parent-commit baseline matches the citations in `implementation.md` (which is byte-identical to `proof-to-rust-map.md` row references):

```
$ diff <(jj file show -r @- crates/vb_ipc/src/frame_types.rs) <(jj file show -r @ crates/vb_ipc/src/frame_types.rs)
41c41
<         let mut cursor = std::io::Cursor::new(&mut bytes[..]);
---
>         let mut cursor = std::io::Cursor::new(bytes.as_mut_slice());
```

The same pattern was confirmed at `seed.rs:23` (parent: `rng.fill(&mut bytes[..]);` → working copy: `rng.fill(bytes.as_mut_slice());`) and `fixture.rs:58` (parent: `rng.fill(&mut vec[..]);` → working copy: `rng.fill(vec.as_mut_slice());`). All three diffs are captured in `.evidence/vb-qol58/verifier/regression-diff.txt`.

## Mandatory Pre-Checks (executed prior to obligation re-runs)

### Verus production-binding pre-check

```bash
$ bash scripts/check-verus-production-binding.sh  # exit 2
ERROR: /verification/verus does not exist
```

Raw exit captured at `.evidence/vb-qol58/verifier/verus-binding-precheck.{log,exit.txt}`.

**Disposition: N/A (no Verus obligation in scope).** The script's design (per the header comment) is to fail fast when it cannot even enumerate Verus specs — i.e., exit 2 indicates "no `verification/verus/` directory → no Verus specs to validate → no VACUUM risk by construction". This bead emits zero `proof-obligation/v1` rows with `verifier: verus` (all 5 verus lane decisions in `verifier-lane-decisions.jsonl` are `not_applicable`; `proof-writer-report.md §"Why 'No Proof Work' Is Honest"` line 4 confirms "auto-satisfied by lane omission"). The script's exit-2 is therefore an honest N/A signal, not a VACUUM blocker. Per the formal-verifier skill workflow step 2 ("Before running any Verus obligation, run ... Any spec file in the VACUUM bucket is a blocker — record FAIL_LOCAL with vacuum_proof as the finding code. Do not run verus on a VACUUM spec; the math verification is irrelevant when the proof is not bound to production") — this rule does not fire because no Verus obligation is in scope.

Note: the script's `git rev-parse --show-toplevel` failure at line 24 is workspace-tooling noise (this isolated JJ workspace is Git-free by design per `AGENTS.md` "Workspace" rules; the same noise is visible in `.evidence/vb-qol58/lint-src.log` and other gate logs without affecting their exit codes).

### Production-inner mirror drift pre-check

```bash
$ bash scripts/check-production-inner-drift.sh  # exit 128
fatal: not a git repository (or any parent up to mount point /)
```

Raw exit captured at `.evidence/vb-qol58/verifier/production-inner-drift-precheck.{log,exit.txt}`.

**Disposition: N/A (no `production_inner/*` mirror exists).** This script's purpose is to detect drift between `verification/verus/production_inner/*.rs` mirrors and their claimed production sources. No Verus spec exists for this bead, so no `production_inner/` mirror exists. The script's exit-128 (`git rev-parse` failure) is workspace-tooling noise identical to the verus pre-check; it does not indicate any drift because there is nothing to drift from. The byte-level drift check below confirms the 3 production lines are correctly refactored:

## Obligation Re-Execution (3 gates)

### PO-qol58-001 — `moon run :lint-src`

| Field | Value |
|---|---|
| Command (planned) | `moon run :lint-src` |
| Command (executed) | `moon run :lint-src` (verbatim) |
| Workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58` |
| Exit status | **0** |
| Raw log | `.evidence/vb-qol58/verifier/lint-src.log` (3569 bytes; SHA-256 `59abb44a322e16f118956bda5cb9c798a2b2d8f8582a9157a93999700ca90b33`) |
| Exit marker | `.evidence/vb-qol58/verifier/lint-src.exit.txt` (`EXIT_CODE=0`) |
| Tool version | moon 2.2.4 (Node-based CLI); underlying clippy via `rustup run nightly-2026-04-28 cargo clippy` (rustc 1.97.0-nightly `52b6e2c20 2026-04-27`) |
| Behavior affecting | false |
| Result | **PASS** |

Sub-tasks visible in the log: `unsafe-audit`, `ignored-fallible-results`, `panic-surface`, `lint-src` (clippy with the 16 deny-list `-D clippy::*` flags from `.moon/tasks/all.yml:51`). Exit status `0` confirms no `panic-surface` violation, no `unsafe-audit` hit, no `-D clippy::*` violation, and `Tasks: 4 completed`. The `moon_task_hasher` warnings on `crates/vb_cli/tests/fixtures/fixtures` are pre-existing tooling noise, not gate failures.

### PO-qol58-002 — `cargo check -p vb_ipc --all-targets --all-features`

| Field | Value |
|---|---|
| Command (planned) | `rustup run nightly-2026-04-28 cargo check --quiet -p vb_ipc --all-targets --all-features` |
| Command (executed) | `rustup run nightly-2026-04-28 cargo check --quiet -p vb_ipc --all-targets --all-features` (verbatim) |
| Workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58` |
| Exit status | **0** |
| Raw log | `.evidence/vb-qol58/verifier/cargo-check.log` (0 bytes due to `--quiet` cache hit; SHA-256 = canonical-empty = `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`) |
| Exit marker | `.evidence/vb-qol58/verifier/cargo-check.exit.txt` (`EXIT_CODE=0`) |
| Tool version | rustc 1.97.0-nightly (`52b6e2c20 2026-04-27`; nightly-2026-04-28 toolchain as pinned in `rust-toolchain.toml`) |
| Behavior affecting | false |
| Result | **PASS** |

The compile-cache emits no output under `--quiet` on a fully-cached compile; exit 0 is the truth that `cargo check` returned no warnings and no errors. The 24-byte IPC header byte layout (`IPC_MAGIC`, `IPC_VERSION`, command, flags, reserved, correlation, payload_len) is preserved verbatim per `implementation.md §"Diffs" diff 1`. The 7 `IpcError::HeaderEncodeFailed` mapping sites at lines 44, 47, 50, 53, 56, 59, 62 are byte-identical pre/post refactor (per `proof-writer-report.md` "production-cite" audit and `proof-findings.jsonl` row 4 `FIND-qol58-PRODUCTION_CITATIONS_VERIFIED`).

### PO-qol58-003 — `cargo test -p velvet-ballistics-workspace-tests --lib --all-features`

| Field | Value |
|---|---|
| Command (planned) | `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` |
| Command (executed) | `rustup run nightly-2026-04-28 cargo test --quiet -p velvet-ballistics-workspace-tests --lib --all-features` (verbatim; package name is the long-form `velvet-ballistics-workspace-tests` per `crates/workspace_tests/Cargo.toml:2`; the user's directive shorthand `workspace_tests` resolves to the same package) |
| Workdir | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-qol58` |
| Exit status | **0** |
| Raw log | `.evidence/vb-qol58/verifier/cargo-test.log` (133 bytes; SHA-256 `bd577d55f236b941832cfce54c469379addf9726f39f5d442594892b2ea25b79`) |
| Exit marker | `.evidence/vb-qol58/verifier/cargo-test.exit.txt` (`EXIT_CODE=0`) |
| Tool version | rustc 1.97.0-nightly (`52b6e2c20 2026-04-27`) |
| Behavior affecting | false |
| Result | **PASS** |

Test summary (verbatim from log):

```
running 18 tests
..................
test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.09s
```

18 ≥ 18 (the user-required threshold). The 7 unit tests named in `PO-qol58-003` `domain_claim` (`seeded_bytes_determinism`, `seeded_bytes_different_seeds`, `seeded_bytes_zero_capacity`, `zero_capacity_rejected`, `valid_capacity_accepted`, `max_capacity_boundary`, `over_max_capacity_rejected`) all live in the test surface and pass — the determinism tests `seeded_bytes_*` exercise `StdRng::seed_from_u64(seed)` → `rng.fill(bytes.as_mut_slice())` byte-for-byte (no semantic diff from `rng.fill(&mut bytes[..])` per `proof-writer-report.md` "byte-equivalent borrow" claim); the capacity tests `*capacity_*` exercise `FixtureBuilder::build_bytes` → `rng.fill(vec.as_mut_slice())` with the seeded RNG preserved end-to-end (no semantic diff).

## Lethal-Finding Scan (post-execution)

| God Rule / Risk | Outcome |
|---|---|
| GOD RULE 1 (No Hardcoded Kani Shapes) | N/A — no `#[kani::proof]` harness written for this bead (kani lane is `not_applicable` for all 5 seeds per `verifier-lane-decisions.jsonl` rows 3, 7, 11, 15, 19; pre-existing `crates/vb_ipc/src/kani_*.rs` harnesses are spelled by the proof reviewer at State 6 as "verifier harnesses, not behavior tests" and continue to cover the panic-freedom surface post-refactor). |
| GOD RULE 2 (No Vacuum Verus Proofs) | N/A — no Verus spec written for this bead (verus lane `not_applicable` for all 5 seeds; production-binding discipline auto-satisfied by lane omission; `scripts/check-verus-production-binding.sh` produces zero `verifier: verus` rows and finds no VACUUM bucket). |
| GOD RULE 3 (No Unbounded TLA+ Math) | N/A — TLA+ is removed per upstream mandate; no TLA+ obligations, lane decisions, or waived lanes. |
| GOD RULE 4 (No Loop Oscillations) | N/A — no proof artifacts were written; no `verus`/`kani`/`flux-rs` model + harness pair exists for this bead; no impl-flaw vs proof-contract oscillation possible. |
| GOD RULE 5 (No Blind Verification Mutations) | PASS — verification scope trimmed to the call-graph blast radius of 3 production lines per `AGENTS.md` and `proof-strategy.md §2`; only the 3 named gates (`moon run :lint-src`, `cargo check -p vb_ipc --all-targets --all-features`, `cargo test -p velvet-ballistics-workspace-tests --lib --all-features`) were re-executed; no new Kani/Verus/Flux/Loom/Miri/fuzz artifacts created. |
| Production-inner drift (`scripts/check-production-inner-drift.sh`) | N/A — no `production_inner/*` mirror exists; the 3 production-line edits cite the actual source paths directly per `proof-to-rust-map.md` table; the re-derived `diff(1)` output above confirms byte-equivalence between the parent baseline and the working-copy post-edit state at all 3 cites. |
| Trusted-base pending disposition (`trusted-base-ledger.jsonl`) | PASS — file is 0 bytes (canonical-empty SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`); no trust markers (`assume`, `axiom`, `admit`, `sorry`, `external_body`, `#[trusted]`, `#[ignore]`, `opaque`, `extern_spec`, stub, cover-only) were introduced. |
| Planned bridge at State 12 closure | PASS — `rust-refinement-obligations.jsonl` is 0 bytes; per `proof-to-rust-review.md STATUS: APPROVED`, zero RROs is the honest disposition for a `behavior_affecting: false` set. |
| `mapping_status` not `planned` | PASS — no `rust-refinement-obligation/v1` rows exist (zero-RRO bead); the closure rule does not apply. |

## Exit-Code Evidence

| Gate | Exit | Marker |
|---|---|---|
| `moon run :lint-src` | 0 | `.evidence/vb-qol58/verifier/lint-src.exit.txt` → `EXIT_CODE=0` |
| `cargo check -p vb_ipc --all-targets --all-features` | 0 | `.evidence/vb-qol58/verifier/cargo-check.exit.txt` → `EXIT_CODE=0` |
| `cargo test -p velvet-ballistics-workspace-tests --lib --all-features` | 0 | `.evidence/vb-qol58/verifier/cargo-test.exit.txt` → `EXIT_CODE=0` |

## Pre-Existing Out-of-Scope Findings (deferred; not blocking)

- **`crates/vb_core/src/lib.rs:26` rustfmt drift** (pre-existing; non-touching site; out of scope for vb-qol58).
- **`crates/vb_runtime/src/shard/transitions.rs` DISCARD-006 justified exception** at lines 199 and 86 (pre-existing; explicit in `scripts/ignored-fallible-results.allow`; accepted by the `panic-surface` and `ignored-fallible-results` sub-tasks of `:lint-src`).
- **`moon_task_hasher` warning on `crates/vb_cli/tests/fixtures/fixtures`** (pre-existing tooling noise; non-failure).

None of the above affects the 3 obligations' verification closure.

## Outputs

- `.beads/vb-qol58/formal-verification-report.md` (this file)
- `.beads/vb-qol58/verification-ledger.jsonl` (3 PASS rows)
- `.beads/vb-qol58/formal-waivers.jsonl` (empty; canonical-empty SHA-256)
- `.beads/vb-qol58/proof-test-source-alignment.jsonl` (3 rows; all `aligned`)
- `.beads/vb-qol58/proof-test-source-alignment.md` (3 rows; all `aligned`)
- `.beads/vb-qol58/regression-diff.md` (3 production-line edits with byte-identical pre-refactor baselines)
- `.beads/vb-qol58/transcript-state12.txt` (verifier transcript)
- `.beads/vb-qol58/agent-invocation-ledger.jsonl` (ledger_sequence 9 appended; parent `holzman-rust-vb-qol58-state11-20260701T192500Z`)
- `.evidence/vb-qol58/verifier/{lint-src.log,cargo-check.log,cargo-test.log,lint-src.exit.txt,cargo-check.exit.txt,cargo-test.exit.txt,jj-diff.txt,regression-diff.txt,verus-binding-precheck.{log,exit.txt},production-inner-drift-precheck.{log,exit.txt}}`

**Formal Verifier Invocation ID:** `formal-verifier-vb-qol58-state12-20260701T225200Z`

**Status:** PASS (3/3 obligations; 0 failures; 0 waivers; 0 trust markers).

STATUS: PASS
