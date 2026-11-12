# State 11 Machine Gate Report

STATUS: APPROVED

## Startup / skill citations

- Read `/home/lewis/.claude/skills/formal-verifier/SKILL.md` and `/home/lewis/.agents/skills/formal-verifier/SKILL.md` before execution.
- Cited formal-verifier rules: lines 21-24 require an approved plan, every obligation accounted, scope-before-status, and fail-closed missing tools.
- Cited execution rules: lines 100-114 require exact obligation commands, recorded exit status/output, no broad all-target clippy style gate, and no silent waivers.
- The `.agents` copy wins on conflict; no relevant conflict was observed.

## Upstream artifact gate

PASS in `/home/lewis/src/vb-qi37-13-r2` only:

```bash
test -s .beads/vb-qi37.13/proof-obligations.jsonl && test -s .beads/vb-qi37.13/traceability-matrix.jsonl && test -s .beads/vb-qi37.13/delivery-scope.jsonl && test -s .beads/vb-qi37.13/baseline-report.md && test -s .beads/vb-qi37.13/tla-spec.md && test -s .beads/vb-qi37.13/lean-contract.md && test -s .beads/vb-qi37.13/contract-verification-review.md && rg -n '^STATUS: APPROVED$' .beads/vb-qi37.13/contract-verification-review.md && jq -c . .beads/vb-qi37.13/proof-obligations.jsonl >/dev/null && jq -c . .beads/vb-qi37.13/traceability-matrix.jsonl >/dev/null && jq -c . .beads/vb-qi37.13/delivery-scope.jsonl >/dev/null
```

Observed output: `3:STATUS: APPROVED`.

## Required obligation gates

See `.beads/vb-qi37.13/verification-ledger.jsonl`. Summary: 9 PASS, 0 FAIL_LOCAL, 0 FAIL_REGRESSION, 0 WAIVED, 0 DEFERRED_GLOBAL.

## Extra State 11 gates requested after black-hat repair

- Structured `DiagnosticReport` JSON/JSONL diagnostics for parse and non-parse routes: PASS, `vb_qi37_13_structured_reconciliation` 11/11.
- stdout/stderr separation: PASS in structured reconciliation tests; success payloads on stdout only, diagnostics on stderr only.
- Public exits `0..=8` / no public `9`: PASS via Verus, exit-code cargo tests, static no-match scan, and CLI agent-context matrix.
- Contracted postcard validation: PASS, `vb_ui_model` postcard 12/12 covering CRC, digest, old/future version, wrong kind, payload bounds before exposure, empty/truncated/header mismatch, bad magic, and roundtrip.
- Verus diagnostic: PASS, `4 verified, 0 errors`.
- GNU fuzz target: PASS, `vb_ui_model_postcard_decode --target x86_64-unknown-linux-gnu -- -runs=1`.
- Command matrix / child reconciliation: PASS, both exact Python checks exited 0/no output.
- Clippy/fmt: PASS for touched CLI package and touched proof/fuzz artifacts.

## Blockers

None. Do not block State 12 on State 11 evidence.
