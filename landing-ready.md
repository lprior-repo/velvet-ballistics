# Landing Ready - vb-0253.1

STATUS: APPROVED

## Bookmark
- bookmark: `go-skill-p0-vb-0253-1`
- approved implementation commit: `7c49a7acbbacd8e6d2fabd6895e408715f5cb0b5`

## Gate Evidence
- `cargo kani -p vb_runtime --harness command_queue_bounds` -> PASS.
- `cargo test -p vb_runtime command_queue -- --nocapture` -> PASS, `11 passed, 1450 filtered out`.
- `cargo check -p vb_runtime` -> PASS.
- `cargo fmt --check` -> DEFERRED_GLOBAL unrelated formatting drift, recorded in `.beads/vb-0253.1/machine-gate-report.md`.

## Stop Point
- State 13 approved and bookmark-ready.
- Main merge intentionally not performed; landing is serialized by master.

---

# Landing Ready - vb-0253.5

STATUS: BOOKMARK_READY

## Evidence Commit

- Commit: `4cec34f0989e4a2b8a794f9cb920f5f320f7cf93`
- Bookmark: `go-skill-p0-vb-0253-5`
- State: 13 APPROVED

## Gates

- Kani: `cargo kani -p vb_core --harness kani_step_state_transition_matches_contract` -> PASS.
- Verus: `verus verification/verus/step_state_machine.rs` -> PASS.
- TLA: `tlc -config specs/tla/StepState.cfg specs/tla/StepState.tla` -> PASS.
- Rust tests: scoped `vb_proof_kernels` and `vb_core` StepState tests -> PASS.
- `cargo fmt --check`: DEFERRED_GLOBAL unrelated formatting drift outside StepState scope.

## Stop Point

Stopped before merging main. Landing is serialized by master.

---

# Landing Ready — vb-qi37.12.2

STATUS: APPROVED

- Bookmark: `go-skill-p0-vb-qi37-12-2`.
- Commit hash at first bookmark push: `e61024e22091`.
- State: 13 / bookmark-ready.
- Merge policy: stop before merging main.

Artifacts verified:

- `.beads/vb-qi37.12.2/final-evidence-decision.md`.
- `.beads/vb-qi37.12.2/truth-serum-report.md`.
- `.beads/vb-qi37.12.2/assurance-bundle.md`.
- `.beads/vb-qi37.12.2/black-hat-review.md`.

---

# Landing Ready — vb-xkli

STATUS: APPROVED

- Bookmark: `go-skill-p0-vb-xkli`.
- Commit hash at first bookmark push: `090213737f89`.
- State: 13 / bookmark-ready.
- Merge policy: stop before merging main.

Artifacts verified:

- `.beads/vb-xkli/final-evidence-decision.md`.
- `.beads/vb-xkli/truth-serum-report.md`.
- `.beads/vb-xkli/assurance-bundle.md`.
- `.beads/vb-xkli/machine-gate-report.md`.

---

# Landing Ready - vb-0253.2

STATUS: APPROVED

## Bookmark

- Bookmark: `go-skill-p0-vb-0253-2`.
- Base main commit: `5ba93c4ddc9375cd85c1d21d5419202d228a9816`.
- Evidence-freeze commit hash before this manifest update: `a61505987b000f81143723005cdaf6cf9513f7a9`.
- Initial pushed bookmark target: `af1b48d30e63f57e544463b916740a648fbfd915`.
- Final bookmark target: verify with `jj log -r go-skill-p0-vb-0253-2`.

## Gate Evidence

- `rtk cargo check -p vb_ipc` -> PASS.
- `rtk cargo test -p vb_ipc` -> PASS, `628 passed`.
- `rtk cargo clippy -p vb_ipc --lib -- -D warnings` -> PASS.
- `cargo kani -p vb_ipc --harness kani_ipc_header_decode_valid --quiet` -> PASS.
- `moon ci` -> FAIL_GLOBAL after main ref repair; out-of-scope `xtask` lint/format, `vb_storage` test warning debt, and `vb_cli` mode-module/import drift only.

## Artifacts

- `.beads/vb-0253.2/final-evidence-decision.md` -> `STATUS: APPROVED`.
- `.beads/vb-0253.2/machine-gate-report.md` -> `STATUS: APPROVED_WITH_GLOBAL_DEBT`.
- `.beads/vb-0253.2/truth-serum-report.md` -> `STATUS: APPROVED`.
- `.beads/vb-0253.2/assurance-bundle.md`.

## Stop Point

- Bookmark-ready and pushed.
- Main merge intentionally not performed.

---

# Landing Ready: vb-qi37.1

STATUS: BOOKMARK_READY

## Bookmark

- Target bookmark: `go-skill-p0-vb-qi37-1`.
- Merge to main: not performed by request.

## Approved Evidence

- State 6 proof-review and contract-verification review approved after current Verus evidence.
- State 11 formal verification approved with exact Verus/TLC/test/Moon task evidence.
- State 12 black-hat review approved.
- State 13 final evidence decision approved.

## Blockers To Fix Outside This Bookmark

- `moon ci` cannot run in this jj workspace because Git cannot resolve `main`.
- `moon run :verify-proof` fails because `scripts/rust-verification-gauntlet.sh` is not valid shell.

---

# Landing Ready: vb-qi37.4

STATUS: BOOKMARK_READY

Bookmark: `go-skill-p0-vb-qi37-4`

## Final State

- Reached State 13.
- Final evidence decision: APPROVED.
- Stopped before main merge by request.

## Verified Artifacts

- `.beads/vb-qi37.4/proof-review.md`
- `.beads/vb-qi37.4/contract-verification-review.md`
- `.beads/vb-qi37.4/test-plan.md`
- `.beads/vb-qi37.4/test-writer-report.md`
- `.beads/vb-qi37.4/test-plan-review.md`
- `.beads/vb-qi37.4/test-suite-review.md`
- `.beads/vb-qi37.4/implementation.md`
- `.beads/vb-qi37.4/machine-gate-report.md`
- `.beads/vb-qi37.4/formal-verification-report.md`
- `.beads/vb-qi37.4/verification-ledger.jsonl`
- `.beads/vb-qi37.4/black-hat-review.md`
- `.beads/vb-qi37.4/assurance-bundle.md`
- `.beads/vb-qi37.4/truth-serum-report.md`
- `.beads/vb-qi37.4/final-evidence-decision.md`
