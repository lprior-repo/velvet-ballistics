# Baseline Report: vb-0253.2

## Git Log (last 5 commits)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Touched Crates/Files (last 5 commits)

### Beads Metadata
- .beads/metadata.json

### Beads State (vb-qi37.1.4, vb-qi37.13)
- .beads/vb-qi37.1.4/STATE.md
- .beads/vb-qi37.1.4/black-hat-review.md
- .beads/vb-qi37.1.4/proof-evidence.md
- .beads/vb-qi37.1.4/proof-obligations.jsonl
- .beads/vb-qi37.1.4/proof-plan-review-input.md
- .beads/vb-qi37.1.4/proof-repair-guide.md
- .beads/vb-qi37.1.4/proof-review.md
- .beads/vb-qi37.1.4/verification-layers.md
- .beads/vb-qi37.13/STATE.md
- .beads/vb-qi37.13/baseline-report.md
- .beads/vb-qi37.13/black-hat-review.md
- .beads/vb-qi37.13/codebase-map.md
- .beads/vb-qi37.13/contract-repair-report.md
- .beads/vb-qi37.13/contract-verification-review.md
- .beads/vb-qi37.13/contract.md
- .beads/vb-qi37.13/defects.md
- .beads/vb-qi37.13/delivery-scope.jsonl
- .beads/vb-qi37.13/domain-model-review.md
- .beads/vb-qi37.13/formal-verification-report.md
- .beads/vb-qi37.13/formal-waivers.candidate.jsonl
- .beads/vb-qi37.13/implementation.md
- .beads/vb-qi37.13/lean-contract.md
- .beads/vb-qi37.13/machine-gate-report.md
- .beads/vb-qi37.13/proof-evidence.md
- .beads/vb-qi37.13/proof-findings.jsonl
- .beads/vb-qi37.13/proof-obligations.jsonl
- .beads/vb-qi37.13/proof-obligations.planned.jsonl
- .beads/vb-qi37.13/proof-plan-review-input.md
- .beads/vb-qi37.13/proof-repair-guide.md
- .beads/vb-qi37.13/proof-review.md
- .beads/vb-qi37.13/proof-strategy.md
- .beads/vb-qi37.13/proof-writer-report.md
- .beads/vb-qi37.13/regression-diff.md
- .beads/vb-qi37.13/test-plan-review.md
- .beads/vb-qi37.13/test-plan.md
- .beads/vb-qi37.13/test-repair-guide.md
- .beads/vb-qi37.13/test-suite-review.md
- .beads/vb-qi37.13/test-writer-report.md
- .beads/vb-qi37.13/tla-spec.md
- .beads/vb-qi37.13/traceability-matrix.jsonl
- .beads/vb-qi37.13/verification-layers.md
- .beads/vb-qi37.13/verification-ledger.jsonl

### Project Root
- CLAUDE.md
- Cargo.lock
- Cargo.toml

### Crates
- crates/vb_runtime/Cargo.toml
- crates/vb_runtime/src/recovery.rs
- crates/vb_storage/Cargo.toml
- crates/vb_storage/src/recovery/recover.rs
- crates/vb_ui_model/src/emitter/binary/tests.rs
- crates/velvet_ballastics/Cargo.toml
- crates/velvet_ballastics/src/cli_postcard.rs
- crates/velvet_ballastics/src/exit_code.rs
- crates/velvet_ballastics/src/main.rs
- crates/velvet_ballastics/tests/vb_qi37_13_structured_reconciliation.rs
- fuzz/Cargo.toml
- fuzz/fuzz_targets.rs
- fuzz/src/bin/vb_ui_model_postcard_decode.rs
- fuzz/src/lib.rs
- verification/verus/diagnostic_envelope_verus.rs

## Known Constraints

- Product/binary/package: velvet-ballastics
- Crate/module prefix: vb_
- Language version: velvet-ballastics/v1
