# Baseline Report — vb-0253.1

## Git Log (HEAD~5..HEAD)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Touched Crates/Files (last 5 commits, .rs + Cargo.toml)

```
crates/vb_runtime/Cargo.toml                     |   1 -
crates/vb_runtime/src/recovery.rs                 |  51 +-
crates/vb_storage/Cargo.toml                      |   1 -
crates/vb_storage/src/recovery/recover.rs         |  23 -
crates/vb_ui_model/src/emitter/binary/tests.rs    |  71 +++
crates/velvet_ballastics/Cargo.toml               |   1 +
crates/velvet_ballastics/src/cli_postcard.rs      | 180 +++++--
crates/velvet_ballastics/src/exit_code.rs         |  30 +-
crates/velvet_ballastics/src/main.rs              | 457 ++++++++++------
.../tests/vb_qi37_13_structured_reconciliation.rs | 580 +++++++++++++++++++++
fuzz/Cargo.toml                                   |   8 +
fuzz/fuzz_targets.rs                              |   5 +
fuzz/src/bin/vb_ui_model_postcard_decode.rs       |  31 ++
fuzz/src/lib.rs                                   |  58 ++-
verification/verus/diagnostic_envelope_verus.rs   | 112 ++--
15 files changed, 1272 insertions(+), 337 deletions(-)
```

## Known Constraints

1. **Bead scope**: runtime shard command queue boundary only; no channel dependency changes
2. **Queue type**: crossbeam_queue::ArrayQueue — bounded, non-alloc on full
3. **Failure modes to avoid**: wrapper becomes generic; full-queue behavior changes
4. **Parent epic**: vb-0253 — Standardize queue and state boundaries
5. **Go-skill lifecycle required**: explore → contract → proof → test → implementation → review → evidence → landing
6. **No unsafe/unwrap/todo/panic permitted per engineering rules**
7. **moon ci is canonical gate**
