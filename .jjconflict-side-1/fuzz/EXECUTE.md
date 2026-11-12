# Fuzz Execution Plan — The Only Document You Need Right Now

> **This is the actionable subset of RED_QUEEN_MASTER_PLAN.md.**
> If you are executing work, read this. If you are planning future work, read RED_QUEEN_MASTER_PLAN.md.
> Phases 2+ are deferred to `fuzz/FUTURE.md`.
> Every step has a failure branch. No happy-path-only instructions.

---

## GATE 0: PRE-FLIGHT CHECK

```bash
# Must pass before any Phase 0 work begins
rustc --version | grep -q nightly || { echo "FATAL: nightly toolchain required"; exit 1; }
cargo --version
which cargo-fuzz || echo "WARN: cargo-fuzz not installed (expected — Phase 0.1 fixes this)"
```

---

## PHASE 0: MAKE IT RUN

**Goal:** Every declared fuzz target compiles with libfuzzer instrumentation, passes 10-second smoke.

**Do not continue past Phase 0 until all smoke tests pass.**

### 0.1 — Install cargo-fuzz

```bash
cargo install cargo-fuzz
cargo fuzz --version
```

**IF FAILS:** Check `rustup component add rust-src`. `cargo-fuzz` requires `rust-src`. If that doesn't fix it, the `fuzz/Cargo.toml` has `edition = "2024"` — `cargo-fuzz` may need a newer version. Try `cargo install --git https://github.com/rust-fuzz/cargo-fuzz`.

### 0.2 — Declare ALL orphan fuzz_targets/ in Cargo.toml

The following 11 files exist in `fuzz/fuzz_targets/` but have no `[[bin]]` entry. Add them:

```
check_doc_taint_consistency_accepts_arbitrary_markdown
decode_record
expr_eval
journal_event
lex_expr
ui_redaction_artifact
vb_5xs4_generated_source_mapping
vb_5xs4_inventory_report
vb_5xs4_label_sufficiency
vb_5xs4_scan_source_text
vb_storage_codec
```

**Warning:** The name `journal_event` collides with the existing `src/bin/journal_event.rs`. The fuzz_targets version must use a different `name =` field. Use suffix `_fuzz`: `name = "journal_event_fuzz"`, `path = "fuzz_targets/journal_event.rs"`.

**IF FAILS:** If a target doesn't compile: check that its `use` statements match the actual crate API. These files may have rotted since they were last compiled. Fix imports, then re-build.

### 0.3 — Declare orphan src/bin/ in Cargo.toml

The following 9 files exist in `fuzz/src/bin/` but have no `[[bin]]` entry:

```
aggregate_artifact_budget
aggregate_workflow_budget
boundary_evidence_reference
boundary_inventory_parser
boundary_metadata
recover_runtime_frame_seed_contract
structured_status_render_hostile
xtask_parse_argv_hostile
xtask_parse_options_hostile
```

Each gets:
```toml
[[bin]]
name = "NAME"
path = "src/bin/NAME.rs"
test = false
doc = false
bench = false
```

**IF FAILS:** These are stdin-based binaries. They may reference `fuzz_lib::fuzz_*` functions that don't exist. Check each `.rs` file for `fuzz_lib::fuzz_SOMETHING` — if the `fuzz_SOMETHING` function doesn't exist in `fuzz/src/lib.rs`, remove the bin file or implement the function.

### 0.4 — Add `[package.metadata.cargo-fuzz]` config

```toml
# In fuzz/Cargo.toml, AFTER the existing [package.metadata] line:
[package.metadata.cargo-fuzz]
libfuzzer_options = [
    "-max_len=65536",
    "-rss_limit_mb=2048",
    "-detect_leaks=1",
]
```

**IF FAILS:** TOML parser rejects combined `[package.metadata] cargo-fuzz = true` and `[package.metadata.cargo-fuzz]` subtable. Keep `cargo-fuzz = true` in `[package.metadata]` and add the subtable separately.

### 0.5 — Add fuzz profile

```toml
# In fuzz/Cargo.toml:
[profile.fuzz]
inherits = "release"
debug = true
debug-assertions = true
overflow-checks = true
lto = "off"
opt-level = 2
codegen-units = 1
```

### 0.6 — Build all targets with libfuzzer

```bash
cargo fuzz build --target x86_64-unknown-linux-gnu --profile fuzz 2>&1 | tee /tmp/fuzz-build.log
```

**IF FAILS:** 
- Check `/tmp/fuzz-build.log` for the first error.
- Common causes: missing crate features (the fuzz crate depends on workspace crates — those crates must compile with their default features), edition mismatches (fuzz crate is edition 2024, some deps may be 2021), missing `rust-src` component.
- If a specific crate fails (e.g., `vb_storage` has known compilation errors), note it and skip that crate's targets. File a bead for the underlying crate fix.
- **Gate:** At least 10 targets must build successfully to proceed. If fewer than 10 build, STOP — the codebase has deeper issues than fuzz infrastructure.

### 0.7 — Verify libfuzzer instrumentation

```bash
BUILD_DIR="fuzz/target/x86_64-unknown-linux-gnu/release"
for bin in $(cargo fuzz list 2>/dev/null); do
    binpath="$BUILD_DIR/$bin"
    if [ -f "$binpath" ]; then
        if nm "$binpath" 2>/dev/null | grep -q LLVMFuzzer; then
            echo "OK: $bin (libfuzzer instrumented)"
        else
            echo "WARN: $bin (no LLVMFuzzer symbols — may be stdin binary, not libfuzzer)"
        fi
    else
        echo "MISSING: $bin ($binpath not found)"
    fi
done
```

**IF FAILS:** `nm` is not installed → `sudo apt-get install binutils`. If libfuzzer targets show no LLVMFuzzer symbols → the build didn't link against libfuzzer-sys. Check that the target uses `#![no_main]` and `fuzz_target!()` macro. Stdin binaries from `src/bin/` will NOT have LLVMFuzzer symbols — this is expected. Only `fuzz_targets/` targets should have them.

### 0.8 — Verify -help=1 works for libfuzzer targets

```bash
for bin in $(cargo fuzz list 2>/dev/null); do
    binpath="$BUILD_DIR/$bin"
    if [ -f "$binpath" ] && nm "$binpath" 2>/dev/null | grep -q LLVMFuzzer; then
        echo "=== $bin ==="
        "$binpath" -help=1 2>&1 | head -3
    fi
done
```

**Expected output per target:** libfuzzer help text, not "command not found" or silent exit.

**IF FAILS:** If a target prints nothing or "Illegal instruction", the binary is corrupt or built for the wrong architecture. Rebuild with `cargo fuzz build --target x86_64-unknown-linux-gnu`.

### 0.9 — First smoke: 10 seconds per target (no sanitizers)

```bash
FAILED_TARGETS=""
for target in $(cargo fuzz list 2>/dev/null); do
    echo "=== SMOKE: $target ==="
    cargo fuzz run "$target" -- -max_total_time=10 -rss_limit_mb=1024 -print_final_stats=1 2>&1 | tail -5
    if [ ${PIPESTATUS[0]} -ne 0 ]; then
        FAILED_TARGETS="$FAILED_TARGETS $target"
        echo "FAIL: $target"
    fi
done
echo "Failed targets: ${FAILED_TARGETS:-none}"
```

**IF FAILS (target crashes):**
- Each crash creates artifacts in `fuzz/artifacts/TARGET/`.
- Minimize: `cargo fuzz tmin TARGET fuzz/artifacts/TARGET/crash-*`
- File a bead with the minimized reproducer.
- If >20% of targets crash, STOP — the codebase has systemic bugs. Do not proceed to hardening until crashes are fixed.
- If 0% crash but some don't compile: document the non-compiling targets in a bead. They are lower priority.

**Gate to proceed to Phase 1:** All compiling targets pass 10-second smoke with zero crashes.

### 0.10 — Second smoke: 10 seconds with ASAN

```bash
RUSTFLAGS="-Zsanitizer=address" \
cargo fuzz build --target x86_64-unknown-linux-gnu --profile fuzz 2>&1 | tail -5

for target in $(cargo fuzz list 2>/dev/null); do
    echo "=== ASAN SMOKE: $target ==="
    cargo fuzz run "$target" -- -max_total_time=10 -rss_limit_mb=1024 -print_final_stats=1 2>&1 | tail -5
done
```

**IF FAILS:** ASAN may find bugs that the unsanitized run missed. Each ASAN finding is a bead. ASAN crashes BLOCK proceeding to Phase 1.

---

## PHASE 1: MAKE IT FIND BUGS

**Goal:** All 21 coverage-only functions hardened with behavioral assertions. Seed corpora created. Fuzz actually proves invariants, not just panic-freedom.

**Prerequisite:** Phase 0.10 must pass (all targets pass 10s ASAN smoke).

### 1.1 — Harden coverage-only functions (21 targets)

For each function listed as "Weak/Coverage-only" in RED_QUEEN_MASTER_PLAN.md §5.1 (lines 333-357), add concrete assertions. The minimum bar is:

1. **Parser/codec targets:** `match result { Ok(v) => { assert!(structural invariant) } Err(e) => { match e { ALL_KNOWN_VARIANTS => {} _ => {} } } }`
2. **Property targets:** Assert at least ONE domain invariant (e.g., `assert!(output.len() <= input.len())`, `assert_ne!(result.type_name(), "unknown")`)
3. **Roundtrip targets:** Assert `encode(decode(bytes))` produces identical bytes OR at minimum `decode(bytes).is_ok() ⇒ decode(bytes) == decode(bytes)` (determinism)

**Hardening checklist per target:**

| # | Target | Action |
|---|--------|--------|
| 1 | `fuzz_yaml_events` | After events collected: assert `!events.is_empty()` for non-empty input. For valid YAML: assert source_map has entries. |
| 2 | `fuzz_replay_events` | After replay: assert `replayed.len() <= events.len()` |
| 3 | `fuzz_extract_terminal` | On Ok: assert `terminal.children().is_empty()` |
| 4 | `fuzz_action_tracker` | After state transitions: assert tracker is in a valid state (enum variant check) |
| 5 | `fuzz_accepted_artifact_envelope_qi37_4_2` | Assert `envelope.accepted_at_seq > 0` |
| 6 | `fuzz_expr_bytecode` | After eval: assert `result.type_name()` is a known type string |
| 7 | `fuzz_verifier_gates` | Per gate result: match error variant to gate's error type |
| 8 | `fuzz_budget_compute` | Assert all budget components are non-negative |
| 9 | `fuzz_admission_flow` | After submit: assert artifact exists in store OR typed error |
| 10 | `fuzz_expr_eval` | Assert eval result has non-empty type_name |
| 11 | `fuzz_accessor_traversal` | Assert traversed path depth ≤ some reasonable bound |
| 12 | `fuzz_admission_fuzz` | After decode: assert parts has at least 1 node |
| 13 | `fuzz_digest_coherence` | Assert `blake3::hash(data) == compute_policy_digest(data)` when both succeed |
| 14 | `fuzz_admission_input_surface` | Assert strict and relaxed paths agree on success/failure |
| 15 | `fuzz_readback_family_set` | Assert classification is one of the known enum variants |
| 16 | `fuzz_accepted_artifact_decode` | Assert decoded artifact has `accepted_at_seq > 0` |
| 17 | `fuzz_recovery_decode` | Assert seed struct has non-zero fields |
| 18 | `fuzz_collect_page_pagination` | **IMPLEMENT THE FUNCTION in lib.rs** — currently does not exist. Must test page_size=0,1,MAX, page count = ceil(list_len/page_size), each page item count ≤ page_size |
| 19 | `fuzz_action_tracker` (src/bin) | Same as #4 |
| 20 | `decode_record` (fuzz_targets) | Replace `.ok()` with `match result { Ok(r) => { assert!(r.is_valid()) } Err(e) => { exhaustive match } }` |
| 21 | `expr_eval` (fuzz_targets) | Same as #10 |

**Template for error-variant exhaustiveness (MANDATORY):**

```rust
match e {
    CrateError::Variant1 { field } => { /* assert on field if meaningful */ }
    CrateError::Variant2 => {}
    CrateError::Variant3 { .. } => {}
    // velvet-zone: error-coverage — when adding a new error variant,
    // add an explicit arm here. The wildcard is for forward-compat only
    // and should NEVER match in current code.
    _ => {}
}
```

**Verification after hardening:** The `_ => {}` arm must NOT be reachable in test. Run a 60-second smoke per target with print_final_stats to confirm corpus growth.

### 1.2 — Fix C.25: Implement collect_page pagination

```rust
// In fuzz/src/lib.rs, add:
pub fn fuzz_collect_page_pagination(data: &[u8]) {
    // Decode a list of items and a page_size
    // Test: collect_page(items, page_size)
    // Assert: page count == ceil(items.len() / page_size)
    // Assert: each page item count <= page_size
    // Assert: page_size=0 → error
    // Assert: empty list → empty result
}
```

### 1.3 — Verify C.21-C.24 fixes with ASAN

```bash
for target in generated_compare compiled_ir ipc_frame expression; do
    echo "=== VERIFY: $target ==="
    cargo fuzz run "$target" -- \
        -max_total_time=3600 \
        -rss_limit_mb=2048 \
        -print_final_stats=1 \
        -detect_leaks=1 2>&1 | tail -10
done
```

**Gate:** All 4 targets must pass 1-hour ASAN with zero crashes and corpus growth > 0. If any crash: fix, bead, re-run.

### 1.4 — Create seed corpora for all targets without them

```bash
# For each target missing corpus:
mkdir -p fuzz/corpus/TARGET/

# Generate seeds from integration test fixtures
# Example: copy a known-valid postcard blob for compiled_ir
# Example: copy a known-valid YAML string for yaml_events
# Example: copy a known-valid IPC frame for ipc_frame

# Hand-craft edge case seeds:
# - Empty input (0 bytes)
# - Single byte (0x00, 0xFF, 0x7F)
# - Magic bytes only (correct and near-miss)
# - Maximum valid input from integration tests
# - Input with one bit flipped from valid
```

**Gate:** Every target has at least 1 seed. Targets with structure-aware inputs have at least 5 seeds.

### 1.5 — Refactor: extract shared stdin boilerplate

```bash
# Create fuzz/src/bin_common.rs with:
# - pub fn run_with_stdin(f: fn(&[u8]))
# - pub fn write_stderr(msg: &str)
# Remove duplicate copies from all 37+ bin files.
# Each bin file becomes:
#   #[cfg(feature = "fuzz")]
#   fn main() -> ExitCode { run_with_stdin(fuzz_lib::fuzz_TARGET) }
```

### 1.6 — Run 1-hour ASAN campaign on all hardened targets

```bash
FAILURES=""
for target in $(cargo fuzz list 2>/dev/null); do
    cargo fuzz run "$target" -- \
        -max_total_time=3600 \
        -rss_limit_mb=4096 \
        -print_final_stats=1 \
        -detect_leaks=1 \
        2>&1 | tee "/tmp/fuzz-1hr-$target.log"
    if [ $? -ne 0 ]; then
        FAILURES="$FAILURES $target"
    fi
done
echo "Failures after 1hr ASAN: ${FAILURES:-ZERO — ALL CLEAR}"
```

**IF FAILS:** Each crash becomes a bead. Fix before proceeding. **Gate to proceed:** Zero crashes, zero leaks, all targets produce final stats showing total_execs > 0.

---

## PHASE 1 EXIT GATE

All of the following must be true before any Phase 2 work begins:

- [ ] `cargo fuzz list` shows all declared targets
- [ ] `cargo fuzz build` succeeds for all targets
- [ ] `nm $BUILD_DIR/TARGET | grep LLVMFuzzer` shows instrumentation for all fuzz_targets/ targets
- [ ] All targets pass 10s smoke (unsanitized)
- [ ] All targets pass 10s smoke with ASAN
- [ ] 21 coverage-only functions hardened with behavioral assertions
- [ ] C.25 collect_page function implemented in lib.rs
- [ ] C.21-C.24 fixes verified with 1-hour ASAN
- [ ] Seed corpora exist for all targets (min 1 seed each)
- [ ] Stdin boilerplate refactored to shared module
- [ ] All targets pass 1-hour ASAN campaign with zero crashes
- [ ] 1-hour campaign logs saved to `/tmp/fuzz-1hr-*.log`

**Phase 1 exit = the fuzz suite is real. It runs. It finds bugs. It doesn't crash on itself.**

Proceed to `fuzz/FUTURE.md` for Phase 2+ (new harnesses, AFL++, CI integration).
