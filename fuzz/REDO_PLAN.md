# Fuzz Campaign — Reverse Prompt / Redo Plan

## What went wrong with the first 12h run

- Ran `cargo build --bins --release` binaries (plain Rust), not libfuzzer targets
- No ASAN sanitizers — memory bugs invisible
- No coverage feedback — every iteration was independent random bytes, no mutation to explore new branches
- `fuzz/src/bin/*.rs` = stdin-reading binaries. `fuzz/fuzz_targets/*.rs` = real libfuzzer harnesses with `#![no_main]` + `fuzz_target!()` macro
- Most fuzz_targets/ files undeclared in Cargo.toml → cargo-fuzz ignores them

## What needs to happen

### Phase 1: Create proper libfuzzer harnesses in `fuzz/fuzz_targets/`

Convert the 6 strongest assertion targets from `fuzz/src/lib.rs` into standalone `fuzz_targets/*.rs` files:

| Lib function | New fuzz_target file | Key invariant |
|---|---|---|
| `fuzz_compiled_ir` | `fuzz_targets/compiled_ir.rs` | Slot bounds, digest preservation, node count |
| `fuzz_journal_event` | `fuzz_targets/journal_event.rs` | Record round-trip, error exhaustiveness |
| `fuzz_ipc_frame` | `fuzz_targets/ipc_frame.rs` | Header round-trip, payload decode, typed errors |
| `fuzz_expression` | `fuzz_targets/expression.rs` | Lex→parse→compile→eval, type invariants |
| `fuzz_taint_propagation` | `fuzz_targets/taint_propagation.rs` | Taint monotonicity, Clean→Clean |
| `fuzz_resource_budget` | `fuzz_targets/resource_budget.rs` | Budget exhaustion, StepBudgetExhausted |

Each file format:
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_compiled_ir(data);  // calls the existing lib.rs function
});
```

### Phase 2: Wire into Cargo.toml

Add `[[bin]]` entries for each new target pointing to `fuzz_targets/`:
```toml
[[bin]]
name = "compiled_ir_fuzz"
path = "fuzz_targets/compiled_ir.rs"
test = false
doc = false
bench = false
```

Use suffixed names (`_fuzz`) to avoid collision with existing `src/bin/` targets.

### Phase 3: Build and verify

```bash
cargo fuzz build compiled_ir_fuzz --target x86_64-unknown-linux-gnu --release
# Verify binary has libfuzzer symbols:
nm fuzz/target/x86_64-unknown-linux-gnu/release/compiled_ir_fuzz | grep LLVMFuzzer
# Test with -help:
./fuzz/target/x86_64-unknown-linux-gnu/release/compiled_ir_fuzz -help=1
# Should show libfuzzer help output, NOT silent
```

### Phase 4: Seed corpus

Create minimal valid inputs for each target so the fuzzer has starting material to mutate:
- `compiled_ir`: a valid `WorkflowParts` postcard blob
- `journal_event`: a valid `JournalEvent` postcard blob
- `ipc_frame`: a valid IPC frame with correct magic + header
- `expression`: a valid YAML expression string
- etc.

### Phase 5: Run with cargo fuzz (12 hours)

```bash
for target in compiled_ir_fuzz journal_event_fuzz ipc_frame_fuzz expression_fuzz taint_propagation_fuzz resource_budget_fuzz; do
  cargo fuzz run "$target" \
    --target x86_64-unknown-linux-gnu \
    --release \
    -- \
    -max_total_time=43200 \
    -print_final_stats=1 \
    &
done
wait
```

This gets us:
- ✅ ASAN (address sanitizer) — catches use-after-free, buffer overflow, stack overflow
- ✅ Coverage-guided mutation — explores uncovered branches, not just random bytes
- ✅ Corpus evolution — saves interesting inputs, mutates them
- ✅ Crash artifacts with reproducers
- ✅ Final statistics: coverage blocks, corpus size, exec/s

### Phase 6: Collect results

```bash
for target in compiled_ir_fuzz journal_event_fuzz ipc_frame_fuzz expression_fuzz taint_propagation_fuzz resource_budget_fuzz; do
  echo "=== $target ==="
  ls fuzz/artifacts/$target/ 2>/dev/null | wc -l  # crash count
  ls fuzz/corpus/$target/ 2>/dev/null | wc -l     # corpus size
done
```

## What the first 12h run proved (still valid)

Despite no ASAN/coverage:
- 171M iterations across 6 targets — zero panics, zero assertion failures
- All structural invariants (slot bounds, digest equality, taint monotonicity) held
- The black-hat review fixes (removed tautologies, removed unreachable!, removed fs::read) are correct at runtime

## What the redo will add

- ASAN detection of memory bugs (use-after-free, OOB writes, leaks)
- Coverage feedback exploring edge cases humans miss
- Corpus-driven mutation finding crashes that random bytes miss
- Proper libfuzzer statistics (coverage, exec/s, corpus growth)
