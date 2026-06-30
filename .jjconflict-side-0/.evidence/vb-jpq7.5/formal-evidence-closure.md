# vb-jpq7.5 formal evidence closure

- Bead: `vb-jpq7.5` — P0 repair formal verification evidence and proof ledgers.
- Workspace: `/home/lewis/src/vb-jpq7-5-formal-evidence-gpt55`.
- Role: `formal-verifier`; proof-reviewer checklist applied in `proof-reviewer-self-check.md`.
- Scope: evidence/ledger closure only. No production Rust, tests, harnesses, or proof artifacts were edited here.

## Related bead status audit

| Bead | Status from `bd show` | Closure/evidence disposition |
|---|---:|---|
| `vb-jpq7.24` | CLOSED | Verus artifact downgraded to mirror-model/non-production evidence; production confidence is via proof-to-Rust bridge and scoped Rust tests. Fresh spot log here: `logs/vb-jpq7-24-verus-spot.txt` (`COMMAND_EXIT=0`, `8 verified, 0 errors`). |
| `vb-jpq7.25` | CLOSED | Kani discovery repair established real crate-local harness discovery and downgraded orphan root `kani/` hardcoded structural files as non-discovered/non-evidence. This closure is superseded for anti-laundering by `vb-jpq7.27` ledger rows. Fresh spot logs here: `logs/vb-core-kani-list-spot.txt` (`COMMAND_EXIT=0`, `Total 143`) and `logs/vb-validate-kani-list-spot.txt` (`COMMAND_EXIT=1`, tracked by `vb-utvm`). |
| `vb-jpq7.26` | CLOSED | Bounded TLA overflow/resource models accepted after typed fail/suspend/full transitions and production journal-full mapping. Fresh spot log here: `logs/vb-jpq7-26-budgetarithmetic-tlc-spot.txt` (`COMMAND_EXIT=0`). |
| `vb-jpq7.27` | CLOSED | Canonical proof-obligation ledger rebuilt with no placeholder/non-evidence PASS rows; external proof-review accepted per bead close reason. Fresh checker log here: `logs/vb-jpq7-27-ledger-check.txt` (`COMMAND_EXIT=0`). |

## Acceptance mapping

### 1. Non-parsing/vacuous Verus files repaired and bound, or marked non-evidence

- `vb-jpq7.24` closed by downgrading `verification/verus/vb_jpq724_events_for_run_production.rs` to mirror-model evidence only. The bridge explicitly says it is **not** direct Verus production-body proof.
- Production linkage is documented in `/home/lewis/src/vb-jpq7-24-verus-binding-gpt55/.evidence/vb-jpq7.24/proof-to-rust-bridge.md` with source refs and scoped tests.
- The canonical `vb-jpq7.27` ledger row `VERUS-VB-JPQ7-24-EVENTS-FOR-RUN-PARSE` limits PASS to parsing/proving the downgraded mirror-model artifact.
- Fresh raw spot command: `logs/vb-jpq7-24-verus-spot.txt`.
- Residual non-proof hygiene: `verusfmt` failure remains child-tracked as `vb-rga1`; it is not laundered as PASS.

### 2. Kani harness discovery lists real harnesses for relevant crates

- Fresh `cargo kani list` in `crates/vb_core` found real crate-local harnesses: `logs/vb-core-kani-list-spot.txt`, `COMMAND_EXIT=0`, `Total 143`.
- `vb-jpq7.25` evidence identifies the old root `kani/` directory as orphaned/non-discovered/non-evidence and points to crate-local Arbitrary/generator harness modules.
- Fresh `cargo kani list` in `crates/vb_validate` fails on missing Arbitrary/generator coverage and a non-exhaustive match: `logs/vb-validate-kani-list-spot.txt`, `COMMAND_EXIT=1`. This is not counted as PASS; it is already child-tracked by `vb-utvm`.

### 3. Hardcoded structural harnesses replaced by generators, or downgraded

- The old root `kani/` hardcoded `WorkflowParts`/`RunFrame` files are downgraded as orphaned/non-discovered/non-evidence in `vb-jpq7.25` and are not in the `vb-jpq7.27` PASS ledger.
- Real PASS discovery is limited to crate-local harnesses that compile under Kani discovery (`vb_core` currently passes; `vb_validate` is blocked and child-tracked).

### 4. Placeholder proof obligations removed from PASS ledgers

- Canonical ledger: `/home/lewis/src/vb-jpq7-27-proof-ledger-gpt55/.evidence/vb-jpq7.27/proof-obligation-ledger.jsonl`.
- Fresh checker: `logs/vb-jpq7-27-ledger-check.txt`, `COMMAND_EXIT=0`, `PASS: vb-jpq7.27 ledger is structurally valid`.
- The checker rejects PASS rows marked `NON_EVIDENCE`, PASS rows with non-zero exit codes, and PASS notes that indicate placeholder/stale-summary evidence.
- Known failures are explicit `FAIL`/`BLOCKED` rows with child beads (`vb-utvm`, `vb-rga1`, `vb-2tpu`), not PASS rows.

### 5. TLA overflow/resource models include typed fail/suspend transitions

- `vb-jpq7.26` acceptance mapping records bounded machine/resource models:
  - Budget add overflow yields `last_result = [tag |-> "Err", error |-> "Overflow"]` and runtime status becomes `"Suspended"` or `"Failed"`.
  - Subtract underflow yields typed `"Underflow"`.
  - Retry exhaustion records typed `"RetryExhausted"`.
  - Lifecycle journal capacity exhaustion records `"JournalFull"` and maps to `RuntimeError::JournalFull { capacity }` without overwrite/drop.
- Fresh BudgetArithmetic TLC spot command: `logs/vb-jpq7-26-budgetarithmetic-tlc-spot.txt`, `COMMAND_EXIT=0`, no invariant/deadlock error.
- The larger `RecoveryReplayFull` model remains unclaimed as PASS and is tracked by `vb-2tpu`.

### 6. Fresh raw logs attached

Fresh logs under this bead workspace:

| Log | CWD | Command | Exit |
|---|---|---|---:|
| `logs/vb-jpq7-27-ledger-check.txt` | `/home/lewis/src/vb-jpq7-27-proof-ledger-gpt55` | `python3 .evidence/vb-jpq7.27/check-ledger.py` | 0 |
| `logs/vb-jpq7-24-verus-spot.txt` | `/home/lewis/src/vb-jpq7-24-verus-binding-gpt55` | `verus verification/verus/vb_jpq724_events_for_run_production.rs` | 0 |
| `logs/vb-jpq7-26-budgetarithmetic-tlc-spot.txt` | `/home/lewis/src/vb-jpq7-26-tla-overflow-gpt55-recovered` | `tlc -metadir /home/lewis/src/vb-jpq7-5-formal-evidence-gpt55/.evidence/vb-jpq7.5/tlc-metadir/BudgetArithmetic -config specs/tla/BudgetArithmetic.cfg specs/tla/BudgetArithmetic.tla` | 0 |
| `logs/vb-core-kani-list-spot.txt` | `/home/lewis/src/vb-jpq7-5-formal-evidence-gpt55/crates/vb_core` | `cargo kani list` | 0 |
| `logs/vb-validate-kani-list-spot.txt` | `/home/lewis/src/vb-jpq7-5-formal-evidence-gpt55/crates/vb_validate` | `cargo kani list` | 1 |

## Residual child-tracked gaps, not vb-jpq7.5 PASS evidence

- `vb-utvm` (OPEN, P0): fix `vb_validate` Kani discovery compile failure; fresh local log confirms `COMMAND_EXIT=1`.
- `vb-rga1` (OPEN, P1): format repaired `vb_jpq724` Verus artifact; not a parse/proof PASS blocker but remains ledger quality work.
- `vb-2tpu` (OPEN, P0): bound/split `RecoveryReplayFull` TLC model; no PASS claim exists.

## Closure decision

`vb-jpq7.5` can close as an evidence/ledger repair bead because the formerly bad evidence is either downgraded to non-evidence, represented by bounded/typed model evidence, or moved to explicit child beads. No known placeholder, hardcoded-shape, stale-summary, non-parsing, or timeout artifact is counted as PASS evidence.
