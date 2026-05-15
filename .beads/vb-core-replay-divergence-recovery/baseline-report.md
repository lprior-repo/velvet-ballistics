# Baseline Report — vb-core-replay-divergence-recovery

## Baseline Git Log (HEAD~5..HEAD)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Touched Crates/Files (last 5 commits)

### Beads
- `.beads/metadata.json` (modified)
- `.beads/vb-qi37.1.4/` (modified — STATE.md, black-hat-review, proof artifacts)
- `.beads/vb-qi37.13/` (added — full planner delivery)

### Crates
- `crates/vb_runtime/` — recovery.rs modified
- `crates/vb_storage/` — `recovery/recover.rs` modified
- `crates/vb_ui_model/src/emitter/binary/tests.rs` modified
- `crates/velvet_ballastics/` — CLI, exit_code, main, new test file
- `fuzz/` — fuzz_targets.rs and new binary modified/added

### Root
- `Cargo.toml`, `Cargo.lock` modified
- `CLAUDE.md` modified

### Verification
- `verification/verus/diagnostic_envelope_verus.rs` modified

## Known Constraints

1. **Dependents blocking this bead:**
   - `vb-core-yaml-e2e-chain` (blocks — e2e chain)
   - `vb-engine-yaml` (blocks — acceptance root)
   - `vb-qi37.1` (blocks — live-frame recovery hydration)

2. **Core requirements:**
   - Restart/replay must NEVER reparses YAML
   - Snapshot+tail must hydrate full frame state
   - Digest mismatch and semantic divergence produce typed errors
   - Corrupt/incomplete frame recovery fails closed

3. **Engineering rules in effect:**
   - No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
   - No YAML/JSON/HTTP in runtime core
   - Generated Rust mode mandatory for maxperf
   - Nightly features gated by `scripts/check-nightly-features.sh`

4. **Source audit note:** storage replay/recovery summaries and digest mismatch checks exist, but object/list slot replay is explicitly unsupported in tests — full RunFrame hydration still needs fail-closed recovery evidence.
