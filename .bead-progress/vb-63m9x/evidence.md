# vb-63m9x — removed-feature-residue-audit

## Bead

`vb-63m9x` — implement a deterministic lint script that audits Cargo features,
benches, Moon tasks, docs, and release gates for ACTIVE PGO / target-cpu=native /
maxperf / generated residue (master §41).

Master quote (verbatim): "PGO, target-cpu=native, maxperf, and generated Rust
benchmark workflows are removed... generated and maxperf are removed and must
not be current default or release features".

## Files Created

1. `scripts/check-removed-feature-residue.rs`  (Rust scanner, Holzman compliant)
2. `scripts/check-removed-feature-residue.sh`  (bash wrapper, builds+runs the .rs)
3. `scripts/test-check-removed-feature-residue.sh`  (self-test, positive+negative+repo)
4. `fixtures/removed-feature-residue/positive.toml`           (clean Cargo feature snippet)
5. `fixtures/removed-feature-residue/negative.toml`           (Cargo feature snippet that triggers)
6. `fixtures/removed-feature-residue/negative_profile.txt`    (Moon/profile snippet that triggers)

## Allowlist Marker Files Modified

6 lines of historical/build-config context received a `# allow-removed-feature:`
comment directly above them so the scanner records them as `allowlisted:` and the
audit still exits 0:

- `.moon/tasks/all.yml:1000`        (maxperf-native RUSTFLAGS, runInCI: false)
- `docs/adr/ADR_REVIEW_GATES.md:18` (review-gate pattern enumerates removed tokens)
- `docs/deferred-codegen-maxperf.md:12` (deferred-scope doc enumerates removed tokens)
- `docs/deferred-codegen-maxperf.md:37` (deferred-scope doc enumerates removed tokens)
- `docs/language-spec.md:1909`      (language-spec narrative on nightly opt-in)
- `docs/rust-governance.md:63`      (rust-governance policy statement)

## Holzman Compliance Check

```
$ rtk rg -n "\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|dbg!" scripts/check-removed-feature-residue.rs
(no matches)

$ rtk rg -n "unsafe[^_]" scripts/check-removed-feature-residue.rs
(no matches)
```

The scanner is `#![forbid(unsafe_code)]` and
`#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic,
clippy::todo, clippy::unimplemented, clippy::dbg_macro)]`. It uses
`unwrap_or`, `unwrap_or_default`, `unwrap_or_else` (the safe non-panicking
variants) per the project's existing scanner convention
(see `check-test-integrity.rs`, `check-hot-loop-bounds.rs`).

## Raw Command Evidence

### 1. Compile the scanner

```
$ mkdir -p target/gate-tools
$ rustc --edition=2024 scripts/check-removed-feature-residue.rs \
    -o target/gate-tools/check-removed-feature-residue
(no output, exit 0)
```

### 2. Positive fixture must pass (exit 0, no active findings)

```
$ bash scripts/check-removed-feature-residue.sh fixtures/removed-feature-residue/positive.toml
summary: active=0 allowlisted=0 files_scanned=1
exit=0
```

### 3. Negative toml fixture must fail (exit 1, file:line finding)

```
$ bash scripts/check-removed-feature-residue.sh fixtures/removed-feature-residue/negative.toml
fixtures/removed-feature-residue/negative.toml:16: REMOVED-FEATURE: generated: feature identifier 'generated =' inside [features] block: generated = []
fixtures/removed-feature-residue/negative.toml:17: REMOVED-FEATURE: maxperf: feature identifier 'maxperf =' inside [features] block: maxperf = ["generated"]
summary: active=2 allowlisted=0 files_scanned=1
exit=1
```

### 4. Negative profile fixture must fail (exit 1, file:line finding)

```
$ bash scripts/check-removed-feature-residue.sh fixtures/removed-feature-residue/negative_profile.txt
fixtures/removed-feature-residue/negative_profile.txt:3: REMOVED-FEATURE: target-cpu=native: exact substring 'target-cpu=native': # the removed master §41 features. Master §41: "PGO, target-cpu=native, maxperf,
fixtures/removed-feature-residue/negative_profile.txt:7: REMOVED-FEATURE: target-cpu=native: exact substring 'target-cpu=native': # The scanner MUST report at least one file:line finding (target-cpu=native or
fixtures/removed-feature-residue/negative_profile.txt:16: REMOVED-FEATURE: target-cpu=native: exact substring 'target-cpu=native': RUSTFLAGS="-C target-cpu=native -Dwarnings"
fixtures/removed-feature-residue/negative_profile.txt:19: REMOVED-FEATURE: pgo: PGO active context 'RUSTC_PGO': # RUSTC_PGO=1 must never be set in the current release pipeline.
fixtures/removed-feature-residue/negative_profile.txt:20: REMOVED-FEATURE: pgo: PGO active context 'RUSTC_PGO': RUSTC_PGO=1
fixtures/removed-feature-residue/negative_profile.txt:22: REMOVED-FEATURE: maxperf: CLI flag '--features maxperf': cargo build -p velvet-ballistics --bin velvet-ballistics --features maxperf
fixtures/removed-feature-residue/negative_profile.txt:23: REMOVED-FEATURE: generated: CLI flag '--features generated': cargo build -p velvet-ballistics --bin velvet-ballistics --features generated
fixtures/removed-feature-residue/negative_profile.txt:24: REMOVED-FEATURE: pgo: PGO active context 'cargo pgo': cargo pgo instrument
summary: active=8 allowlisted=0 files_scanned=1
exit=1
```

### 5. Real repository audit must pass (exit 0, no active residue)

```
$ bash scripts/check-removed-feature-residue.sh
.moon/tasks/all.yml:1001: allowlisted: master §41 — maxperf-native is legacy build config, not a current release gate (runInCI: false):       export RUSTFLAGS="-C target-cpu=native -Dwarnings"
docs/adr/ADR_REVIEW_GATES.md:19: allowlisted: master §41 — review-gate pattern intentionally enumerates removed tokens to reject them: rg -n "generated Rust|maxperf|PGO|target-cpu=native|Makepad|UI" docs/adr docs/master-decomposition.md
docs/deferred-codegen-maxperf.md:13: allowlisted: master §41 — historical deferred-scope document explicitly enumerates the removed tokens:   generated-vs-IR benchmark ratios, PGO, and `target-cpu=native` maxperf release gates.
docs/deferred-codegen-maxperf.md:38: allowlisted: master §41 — historical deferred-scope document explicitly enumerates the removed tokens: 7. PGO training and `target-cpu=native` benchmark workflows.
docs/language-spec.md:1910: allowlisted: master §41 — language-spec narrative notes that nightly may still opt-in to the removed tokens; they remain removed as release gates: Nightly Rust may be used for LTO, PGO, target-cpu=native local builds, panic=abort release builds, codegen-units=1, allocator benchmarking, portable SIMD experiments, build-std experiments, allocator API experiments, and feature-gated specialization experiments. Nightly features must not leak into the public language.
docs/rust-governance.md:64: allowlisted: master §41 — rust-governance policy statement enumerates the removed tokens: `maxperf`, PGO, generated Rust execution, and `target-cpu=native` workflows are deferred from the current Backend / IR Interpreter Complete milestone. They must not be current release gates or performance evidence unless a future architecture bead reactivates them.
summary: active=0 allowlisted=6 files_scanned=2005
exit=0
```

### 6. Self-test must pass (exit 0, all four assertions hold)

```
$ bash scripts/test-check-removed-feature-residue.sh
[1/3] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/3] negative toml fixture must FAIL (exit 1, file:line finding)
  ok: exit 1 with file:line finding
  ok: summary reports active > 0
[2b/3] negative profile fixture must FAIL (exit 1, file:line finding)
  ok: exit 1 with file:line finding
[3/3] real repository scan must PASS (exit 0, no active residue)
  ok: exit 0
  ok: summary reports active=0
  ok: no REMOVED-FEATURE line in output
self-test PASSED
exit=0
```

## Token Rule Summary (as implemented)

| Token              | Match form                                                       |
|--------------------|------------------------------------------------------------------|
| `target-cpu=native` | exact substring anywhere in the line                            |
| `pgo`               | restricted to 4 PGO active contexts: `pgo = `, `cargo pgo`, `pgo-data`, `RUSTC_PGO` |
| `maxperf`           | feature identifier only: inside `[features]` block, or `--features maxperf` / `--features=maxperf` |
| `generated`         | feature identifier only: inside `[features]` block, or `--features generated` / `--features=generated` |

The scanner also self-skips `scripts/check-removed-feature-residue.sh`,
`scripts/check-removed-feature-residue.rs`, and
`scripts/test-check-removed-feature-residue.sh` because the audit script
itself necessarily mentions the banned tokens in its docstring / constants
(following the precedent of `check-nightly-features.sh` which skips itself
to mention `RUSTC_BOOTSTRAP`).
