# Proof Review: vb-2b4g

## Findings

- No blocking proof-evidence findings remain.

## Reviewed Evidence

- Startup skill loaded: `proof-reviewer`.
- Prior rejection reviewed: stale `PO-007` mismatch in `.beads/vb-2b4g/proof-review.md` and `.beads/vb-2b4g/proof-findings.jsonl`.
- Required current inputs read: `.beads/vb-2b4g/proof-obligations.jsonl`, `.beads/vb-2b4g/formal-verification-report.md`, `.beads/vb-2b4g/verification-ledger.jsonl`, `.beads/vb-2b4g/machine-gate-report.md`, and `.beads/vb-2b4g/formal-waivers.jsonl`.
- JSONL sanity gate run from `/tmp/opencode/go-skill-vb-2b4g`: `pwd -P && test -s .beads/vb-2b4g/proof-obligations.jsonl && jq -c . ... >/dev/null` — PASS.
- Exact `PO-007` verifier rerun from `/tmp/opencode/go-skill-vb-2b4g`:

  ```bash
  /home/lewis/.cargo/bin/cargo check -p vb_codegen --all-targets && /home/lewis/.cargo/bin/cargo test -p vb_codegen --test trybuild_tests && /home/lewis/.cargo/bin/cargo fmt --all -- --check
  ```

  Raw result: cargo check finished dev profile in 0.07s; trybuild test binary ran 3 tests and all passed, including non-empty pass and compile-fail fixtures; `cargo fmt --all -- --check` completed with no diff output.

## Coverage Assessment

- `PO-001` through `PO-006`: accounted in `.beads/vb-2b4g/verification-ledger.jsonl:1-6` as executable PASS results for focused runtime parity, oracle guard, and generated-source static scan.
- `PO-007`: repaired and approved. `.beads/vb-2b4g/verification-ledger.jsonl:7` now records one `PO-007` record with the declared command and direct cargo-binary equivalent, result `PASS`; the reviewer reran the same direct cargo-binary command successfully.
- `PO-008`: honestly classified as `DEFERRED_GLOBAL`, not PASS. `.beads/vb-2b4g/machine-gate-report.md:20-42` and `.beads/vb-2b4g/verification-ledger.jsonl:8` cite disk-quota/resource failures after scoped local `vb_codegen` gates passed.
- Formal/non-claimed lanes: `.beads/vb-2b4g/formal-waivers.jsonl:1-9` provides explicit `WAIVED` or `NOT_IN_SCOPE` classification guidance. `.beads/vb-2b4g/verification-ledger.jsonl:10-18` keeps TLA+/formal-state-machine/Verus/Kani/Lean/Aeneas/Hax/theorem/performance out of PASS status.

## Residual Risks

- Runtime parity evidence remains executable-test confidence only, not formal refinement proof.
- `moon ci` remains deferred until disk/quota is remediated and the workspace gate is rerun successfully.
- No performance, TLA+, Verus, Kani, Lean/Aeneas/Hax, theorem-kernel, or formal state-machine proof may be claimed from this bead.

STATUS: APPROVED
