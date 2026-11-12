# Test Review — vb-xhrt9 (independent DEEPSEEK reviewer)

**Verdict: REJECT** — one specific defect; not a coverage failure.

## What is adequate (does catch the V1 masks)

`.beads/vb-e7rjr/supply-chain-status-selftest.sh` uses tiny fake report commands and asserts persisted status values for the required code set:

- Advisory capture: 0 (`geiger-ok`), 1 (`deny-fail-1`), 2 (`vet-fail-2`), 124 (`machete-timeout`), 127 (`tool-not-found`).
- Audit fail-closed: pass (0) and fail 1 / fail 3, asserting the script exits with the same code.

This satisfies the V1 rejection point (5) "test shell with tiny FAKE REPORT commands exit 0/1/2/124/127 and audit fail; verify persisted statuses EXACT and all report commands continue, no whole || true". Fresh run: exit 0, `PASS: 8 FAIL: 0` (`.beads/vb-xhrt9/evidence/selftest-fresh.json`, and author's `capture-selftest-v2.json`).

## Exact defect — handwritten substitute, not extracted real Moon script

The requirement is that the selftest **exercise the extracted real Moon script, not a handwritten substitute only**. It does not:

1. `run_advisory_report` is copy-pasted into the selftest (comment claims "exact ... from production supply-chain script"), but there is no extraction from `.moon/tasks/all.yml`. A future production edit would not be caught — the selftest would keep testing the frozen copy.
2. The audit block is a **rewrite**, not the production block. Selftest uses `timeout 1s cargo --version ...`; production uses `timeout 2m rustup run nightly-2026-04-28 cargo audit --quiet ...` and additionally emits the "found vulnerabilities" message and `cat`s `audit.out`/`audit.err`. Only the fail-closed exit propagation is exercised; the production audit invocation string is never tested.
3. The 124 and 127 cases are simulated by fixtures that `exit 124` / `exit 127`, not by an actual `timeout`-induced 124 or a real missing-tool 127. Acceptable for synthetic coverage, but it means the selftest never runs the real tool invocation path.

Net: the matrix is a faithful-but-frozen handwritten substitute. It protects against regression in the copied logic only; it cannot detect drift between the selftest and the production script.

## Recommended fix (author, not reviewer)

Rebuild the selftest to extract the `supply-chain` `script` block from `.moon/tasks/all.yml` (e.g., the block after `  supply-chain:` → `    script: |`) and drive it with PATH fixtures (`rustup` shim keyed on `audit|deny|vet|geiger|machete`), asserting per-tool `.status` values and the task exit code for (a) all-advisory-nonzero → 0, (b) audit-nonzero → same code. This is exactly the extraction the reviewer used to independently verify the production script (see `code-review.md`), so it is known to work.

## Independent confirmation of the real script

The reviewer already extracted and executed the **actual** production script and confirmed: no `|| true`/`set +e` masking, no top-level `return`, real `run_advisory_report` captures 0/1/2/124/127, all advisory failures continue (task exit 0), and `cargo audit` nonzero fails the task (exit 1 and 3). So the production code is correct (code-review APPROVE); the REJECT here is scoped to the selftest's failure to test that code via extraction.

No production source edits. No proof-completion or speed claims.
