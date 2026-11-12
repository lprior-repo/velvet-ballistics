# Extreme Rust Language Fuzzing — velvet-ballistics Doctrine

> **Derived from multi-engine fuzzing research across libFuzzer, AFL++, honggfuzz, LibAFL, NAUTILUS, Superion, Csmith, Rustlantis, Fuzzilli, and JITfuzz.**
> 
> For language tooling, the highest-yield strategy is not a single fuzzer or a single corpus. It is a layered campaign: start with a libFuzzer-compatible in-process harness via `cargo-fuzz`; then drive the same target through AFL++ for long-running persistent campaigns and stronger mutational machinery; honggfuzz for alternate feedback and crash monitoring; LibAFL only when you need custom schedulers, custom feedback, binary-only backends, or distributed scaling.
>
> On top of that, use `arbitrary` or `fuzzcheck` to stop wasting cycles on obviously malformed inputs, and use `proptest` to encode shrinking-friendly semantic invariants that become long-term regression tests.

---

## 1. Engine Layering

| Engine | Role in velvet-ballistics |
|--------|--------------------------|
| **cargo-fuzz + libFuzzer** | Primary harness development, parser/front-end, local triage. Struct-aware via `Arbitrary`. |
| **AFL++ via afl.rs** | Long-haul campaigns, persistent loops, compare-heavy grammars (YAML tokens, IPC magic bytes), corpus farming |
| **honggfuzz** | Alternate-feedback lane (hardware counters), crash-heavy targets, multi-process execution |
| **LibAFL** | Custom AST/IR mutators for `vb_compile` and `vb_codegen`, distributed scaling if needed |
| **fuzzcheck** | Structure-aware fuzzing for recursive Rust data types (`WorkflowParts`, `CompiledNodeKind`) |
| **arbitrary** | Typed bridge from fuzzer bytes to `WorkflowParts`, `JournalEvent`, `IpcPayload`, `ExprOp` |
| **proptest** | Shrinking-friendly semantic invariants for CI regression testing |

### Installation (Linux x86_64 baseline)

```bash
rustup toolchain install nightly --component llvm-tools-preview
cargo install cargo-fuzz cargo-afl honggfuzz cargo-llvm-cov cargo-fuzzcheck
```

---

## 2. Mutation Strategies That Reach Deep Language States

The central mistake is to fuzz only bytes. Byte mutation is mandatory but only the bottom layer. The most effective campaigns combine multiple strategies:

| Strategy | velvet-ballistics target | Rust implementation |
|----------|-------------------------|---------------------|
| **Coverage-guided byte mutation** | `fuzz_yaml_events`, `fuzz_ipc_frame`, `fuzz_journal_event` | libFuzzer default |
| **Compare-guided + dictionary-driven** | Magic bytes (VBRT, VBCA, VBIPC), YAML keywords | libFuzzer `-dict`, AFL++ CMPLOG |
| **Structure-preserving typed mutation** | `WorkflowParts`, `CompiledWorkflow`, `ExprOp` | `arbitrary` + `fuzzcheck` |
| **Grammar-aware generation** | YAML source → AST → IR pipeline | fuzzcheck grammar mutators, LibAFL custom mutators |
| **Corpus distillation** | All long-running campaigns | `cargo fuzz cmin`, `afl-cmin` |
| **Differential oracles** | Optimized vs unoptimized eval, YAML events vs AST, admission paths | proptest + libFuzzer |

### Three Concurrent Lanes

```
LANE 1: Raw byte/coverage-guided → lexer, parser, file/container boundaries
LANE 2: Typed/grammar-aware → parser + semantic checker (validate gates 07-15)
LANE 3: IR/sequence-aware → vb_compile lowering, vb_codegen emit, vb_runtime primitives
```

---

## 3. Stage-Split Harnesses

Do not only fuzz "compile and run everything." Build separate narrow entrypoints per language stage. libFuzzer explicitly recommends narrower targets.

### velvet-ballistics Stage Harnesses

| Stage | Harness | What it fuzzes | Oracle |
|-------|---------|---------------|--------|
| **Lexer** | `fuzz_expr_lex` | `vb_expr::lex()` on raw bytes | Token stream validity, never panics |
| **Parser** | `fuzz_expr_parse` | `vb_expr::parse()` on token streams | AST structure invariants |
| **YAML→IR** | `fuzz_yaml_compile` | `vb_yaml::collect_events()` → `vb_compile::compile_workflow()` | node_count ≥ 1, source_map non-empty |
| **IR validation** | `fuzz_compiled_ir` | `postcard::from_bytes::<WorkflowParts>()` → `try_from_parts()` | Digest stability, slot bounds, node count |
| **Expression eval** | `fuzz_expression` | Lex→parse→compile→evaluate | Taint monotonicity, Clean→Clean |
| **Expression diff** | `fuzz_expr_differential` | Direct eval vs compile+eval | Same result both paths |
| **Bytecode** | `fuzz_expr_bytecode` | `ExprOp` sequence execution | Stack invariants, type_name non-empty |
| **Codegen** | `fuzz_codegen_compare` | `emit_rust_workflow()` vs `try_from_parts()` | Digest/node/slot equality |
| **IPC decode** | `fuzz_ipc_frame` | `decode_frame_header()` + `decode_frame_payload()` | Magic validation, length bounds, payload ≤ header |
| **Storage codec** | `fuzz_journal_event` | `decode_record::<JournalEvent>()` | Roundtrip equality, is_valid() |
| **Admission** | `fuzz_admission_surface` | `submit_artifact()` | Artifact store invariants |
| **Validation gates** | `fuzz_verifier_gates` | All 9 gates (07-15) | Per-gate error variant exhaustiveness |
| **Runtime primitives** | `fuzz_collect_page`, etc. | `collect_page()`, `for_each()`, `retry()` | Pagination math, budget exhaustion |
| **Taint** | `fuzz_taint_propagation` | Full workflow taint propagation | Monotonicity, Clean→Clean |

---

## 4. Multi-Oracle Harness Template

```rust
#![no_main]

use libfuzzer_sys::{arbitrary::Arbitrary, fuzz_target};

#[derive(Debug, Arbitrary)]
struct ProgramInput {
    source: String,
    optimize: bool,
    fuel: u32,
}

fuzz_target!(|input: ProgramInput| {
    let fuel = input.fuel.min(50_000);

    // Stage 1: parser
    if let Ok(ast) = my_lang::parse_module(&input.source) {
        // Stage 2: round-trip oracle (parse → print → parse)
        let printed = my_lang::print_module(&ast);
        let reparsed = my_lang::parse_module(&printed)
            .expect("printer must emit parseable source");
        assert_eq!(ast, reparsed, "parse/print/parse mismatch");

        // Stage 3: compile (must not panic)
        let bytecode = my_lang::compile_module(&reparsed, input.optimize)
            .expect("compiler must return Result, not panic");

        // Stage 4: bounded execution with fuel
        let mut vm = my_lang::Vm::with_fuel(fuel as usize);
        let _ = vm.run(&bytecode);

        // Stage 5: differential oracle (optimized vs unoptimized equivalence)
        let a = my_lang::eval_source(&input.source, false).ok();
        let b = my_lang::eval_source(&input.source, true).ok();
        assert_eq!(a, b, "optimization changed observable behavior");
    }
});
```

### velvet-ballistics Oracles

| Oracle type | velvet-ballistics application |
|-------------|------------------------------|
| **Round-trip** | `encode(decode(x)) == x` for postcard, IPC frames, journal records |
| **Differential** | Direct eval ≡ compile+eval; YAML events ≡ YAML compile; admission paths agree |
| **Metamorphic** | `collect_page(list, N)` → page count = ceil(len/N); `taint(output) ≥ max(taint(inputs))` |
| **Budget** | Zero budget → zero transitions; executed ≤ budget; step_budget clamping |
| **Validation** | `validate_compiled_workflow(w)` ≡ `try_from_parts(parts)` agreement |
| **Determinism** | `decode(decode(x).encode()) == decode(x)` for all codecs |

---

## 5. Sanitizer Matrix

Instrument as a matrix, not a single mode.

| Sanitizer | When to use | velvet-ballistics relevance | Cargo flag |
|-----------|------------|---------------------------|------------|
| **ASan + LSan** | Default workhorse — unsafe Rust, FFI, allocator misuse | ALL targets | `-Zsanitizer=address` |
| **UBSan** | Integer overflow, null deref, misaligned access (C/C++ side only) | Not available in Rust natively; use `overflow-checks = true` instead | N/A for Rust |
| **MSan** | Uninitialized reads (requires MSAN-instrumented stdlib — practically impossible in Rust) | NOT RECOMMENDED for Rust. Use Miri for uninit detection. | `-Zsanitizer=memory` |
| **TSan** | Parser/interpreter/compiler races | `vb_runtime::ActionQueue`, `vb_ipc` server, shard lifecycle | `-Zsanitizer=thread` |
| **Miri** | Minimized reproducers involving unsafe Rust | velvet-ballistics has ZERO unsafe blocks — low priority | `cargo miri test` |
| **Source coverage** | Corpus quality, harness blind spots, CI reporting | All targets after campaigns | `cargo fuzz coverage` |

### Recommended velvet-ballistics sanitizer lanes

```
LANE 1 (every target): ASan + LSan (default with cargo-fuzz)
LANE 2 (concurrency targets): TSan on vb_runtime action_queue, vb_ipc, shard
LANE 3 (coverage): cargo-llvm-cov after each nightly campaign
```

---

## 6. Corpus Management

```
SEED → FUZZ → MINIMIZE → MERGE → DISTILL → REGRESSION
```

| Tool | Command | Purpose |
|------|---------|---------|
| Seed generation | `cp fixtures/valid.bin fuzz/corpus/TARGET/` | Starting material |
| Minimize crash | `cargo fuzz tmin TARGET fuzz/artifacts/TARGET/crash-*` | Smallest reproducer |
| Minimize corpus | `cargo fuzz cmin TARGET` | Remove redundant seeds |
| Merge corpora | `cargo fuzz run TARGET -- -merge=1 fuzz/corpus/TARGET/` | Deduplicate across workers |
| AFL++ minimize | `afl-cmin -i in -o out -- ./TARGET` | AFL-native minimization |
| Coverage report | `cargo fuzz coverage TARGET && cargo llvm-cov report` | Human diagnosis |

---

## 7. Crash Handling Pipeline

```
CRASH → REPRODUCE → MINIMIZE → BUCKET → REGRESS → FIX → RE-FUZZ
```

### Bucketing (not filename hashes)

Bucket by `sanitizer + top_frame + program_phase`:

```
ASan:vb_storage::codec::decode_record_header
UBSan:vb_core::engine::eval_expr_with_store
TSan:vb_runtime::action_queue::enqueue
Timeout:vb_yaml::collect_events
Leak:vb_ipc::server::dispatch
```

### Regression test format

Every crash becomes a deterministic test in `crates/workspace_tests/tests/`:

```rust
#[test]
fn regression_fuzz_crash_20260524_asan_decode_record() {
    let input = include_bytes!("../../fuzz/regressions/decode_record/20260524-asan-crash-001.bin");
    let result = vb_storage::codec::decode_record::<vb_storage::JournalEvent>(
        input,
        vb_storage::MAGIC_JOURNAL_EVENT,
        vb_storage::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
    );
    // Must not panic, must not hang, must return a typed error or valid event
    match result {
        Ok((_env, event)) => assert!(event.is_valid()),
        Err(e) => match e {
            vb_storage::JournalError::PostcardDecodeFailed => {}
            vb_storage::JournalError::PayloadDigestMismatch { .. } => {}
            // velvet-zone: error-coverage — all known variants listed
            _ => {}
        },
    }
}
```

---

## 8. Campaign Monitoring

### Minimum Live Dashboard Metrics

| Metric | Tool | Why |
|--------|------|-----|
| Executions/second | libFuzzer `-print_final_stats=1` | Throughput baseline |
| Current RSS | libFuzzer stats | Memory pressure |
| Edges found | libFuzzer stats, AFL++ `edges_found` | Coverage growth |
| Corpus count | `ls fuzz/corpus/TARGET/ \| wc -l` | Corpus diversity |
| Unique crashes | `ls fuzz/artifacts/TARGET/ \| wc -l` | Bug discovery rate |
| Time since last new edge | libFuzzer stats | Plateau detection |

### Plateau Response

If no meaningful new edges, states, or buckets for 24-72 hours:
1. Switch from byte mutation to structure-aware (`Arbitrary` impls)
2. Add differential oracles (optimized vs unoptimized, direct vs compiled eval)
3. Add grammar-aware dictionaries (YAML tokens, IPC magic bytes, expression operators)
4. Switch engines (libFuzzer → AFL++ for deterministic stage)
5. Write a narrower harness (stage-split from monolithic to per-stage)

---

## 9. velvet-ballistics Campaign Timeline

```
PHASE 0 (COMPLETE): Foundation
  └── cargo-fuzz installed, 55 targets compile, libfuzzer instrumentation confirmed

PHASE 1 (NOW): Harden + Seed
  ├── Harden 21 weak functions with behavioral assertions
  ├── Fix C.25 collect_page pagination
  ├── Verify C.21-C.24 with 1-hour ASAN
  ├── Create seed corpora for all targets
  └── Refactor stdin boilerplate

PHASE 2: Structure-Aware
  ├── impl Arbitrary for WorkflowParts, JournalEvent, IpcPayload, ExprOp
  ├── Create grammar-aware harnesses (fuzzcheck for recursive types)
  ├── Extract token dictionaries (YAML keywords, IPC magic, expression operators)
  └── Build differential oracle harnesses (direct vs compile+evaluate)

PHASE 3: Multi-Engine
  ├── AFL++ persistent-mode corpus farming on top-10 targets
  ├── honggfuzz hardware-feedback lane on top-5 compute-heavy targets
  ├── Cross-engine corpus merge and distillation
  └── Coverage gap analysis (Fuzz Introspector style)

PHASE 4: Depth
  ├── Stage-split harnesses (lexer-only, parser-only, checker-only, VM-only)
  ├── Differential compiler lane (YAML events vs AST vs compiled IR equivalence)
  ├── Stateful sequence fuzzing (shard lifecycle, action queue, timer wheel)
  └── JIT/optimization lane (compile with multiple optimization levels, compare outputs)

PHASE 5: Perpetual
  ├── Nightly ASan campaign on all targets
  ├── Weekly TSan campaign on concurrency targets
  ├── Monthly mutation testing on harnesses (≥90% kill rate)
  ├── Regression library: every crash → minimized reproducer → deterministic test
  └── ClusterFuzzLite continuous fuzzing
```

---

## 10. Non-Negotiable Rules

1. **Every parser/codec gets a harness.** `&[u8]` → typed Result → assertions on both arms.
2. **Stage-split, don't monolith.** Narrow targets per language stage reach deeper states.
3. **Explicit quotas in the system under test.** Recursion depth, step count, fuel/gas, heap growth, output size. This is core fuzz-resistant design.
4. **Multi-oracle.** Crash-only fuzzing leaves miscompilations and semantic bugs invisible. Add round-trip, differential, and metamorphic oracles.
5. **Sanitizer matrix.** ASan default, TSan for concurrency, Miri for unsafe reproducers.
6. **Corpus discipline.** Distill regularly, but save semantically rich seeds before minimizing.
7. **Every crash → regression test.** File-based replay with deterministic assertions.
8. **Plateau = signal.** No new edges in 24-72 hours → switch lanes, add grammar, add oracles.
9. **`_ => {}` only with velvet-zone comment.** Error variant wildcards must document why.
10. **libFuzzer `-1` return for stage gates.** When a later-stage harness only wants inputs that pass an earlier stage, use libFuzzer's reject convention.

---

## References

- [Rust Fuzz Book](https://rust-fuzz.github.io/book/) — cargo-fuzz, libFuzzer, structure-aware fuzzing
- [AFL++](https://github.com/AFLplusplus/AFLplusplus) — afl.rs, persistent mode, CMPLOG
- [honggfuzz-rs](https://github.com/rust-fuzz/honggfuzz-rs) — hardware-assisted feedback
- [LibAFL](https://github.com/AFLplusplus/LibAFL) — modular fuzzing framework
- [fuzzcheck](https://github.com/loiclec/fuzzcheck-rs) — grammar-aware Rust fuzzer
- [arbitrary](https://github.com/rust-fuzz/arbitrary) — typed bridge from bytes to Rust values
- [proptest](https://github.com/proptest-rs/proptest) — property testing with shrinking
- [NAUTILUS](https://www.syssec.ruhr-uni-bochum.de/research/publications/nautilus/) — grammar-aware fuzzing
- [Superion](https://github.com/zhunki/Superion) — AST-aware trimming and mutation
- [Csmith](https://github.com/csmith-project/csmith) — differential compiler testing
- [Rustlantis](https://github.com/rust-lang/rustlantis) — MIR-level Rust compiler fuzzing
- [Fuzzilli](https://github.com/googleprojectzero/fuzzilli) — JIT-specific fuzzing IR
- [JITfuzz](https://github.com/sslab-gatech/jitfuzz) — optimization-activating JIT mutators
