# Error Taxonomy — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Scope

`Runtime::collect_metrics` does not return a `Result` and never raises domain errors. The only failure modes in the path are:

1. Lint diagnostics emitted by `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` if the replacement reintroduces an `as`-cast.
2. AST-scanner diagnostics emitted by `xtask forbidden-scan` if an `as`-cast reappears anywhere in `vb_runtime` production source.
3. Numeric regression failures emitted by `cargo test -p vb_runtime --lib trace_ring_fill_pct` if the replacement diverges from the documented equivalence class.
4. Behavioural NaN in the (unreachable) hypothetical where `TraceRing::new`'s `capacity.max(1)` clamp is ever violated.

## Domain Error Variants

None. The fix preserves the `RuntimeMetricsSnapshot`-returning signature.

## Lint / Tooling Errors (Behaviour-Affecting? No)

| Error code / source | Severity | Meaning | Behaviour-affecting? | Owner |
|---------------------|----------|---------|----------------------|-------|
| `clippy::as_conversions` (`runtime.rs:580-588`) | Error (CI gate `-D`) | A `as`-cast is present in production source where the workspace `as_conversions = "deny"` lint forbids it | Yes — must be fixed before landing | `holzman-rust` / `functional-rust` |
| `forbidden-scan` AST scanner (`docs/master/section-041-forbidden-scan-contract.md:26`) | Error | The AST scanner found an `as`-cast in first-party `vb_runtime` source | Yes — must be fixed before landing | `black-hat-reviewer` |
| `cargo test trace_ring_fill_pct` (3 RA-003 tests) | Test failure | The replacement diverges from the pinned `f32`-vs-`f64` numerical equivalence class for some `trace_capacity ∈ [1, 2^20]` | Yes — must be fixed before landing | `test-writer` |
| `cargo fmt --check` / `cargo clippy --all-targets` (general lint) | Error | The replacement is otherwise lint-clean but introduced a non-`as_conversions` lint | Yes — must be fixed before landing | `holzman-rust` |
| `cargo build` compile error | Error | The replacement does not compile (e.g. `unwrap_or` fallback type mismatch) | Yes — must be fixed before landing | `holzman-rust` |

## Runtime Errors (Behaviour-Affecting? No)

| Error | Description | How it surfaces |
|-------|-------------|-----------------|
| Hypothetical `0.0 / 0.0 = NaN` | If `TraceRing::new`'s `capacity.max(1)` clamp is violated upstream, the `unwrap_or(0)` fallback would yield `0.0 / 0.0 = NaN`, which `f32::is_nan` would report as `true`. The IPC roundtrip test `shard_metrics_with_nan_trace_ring_fill_pct_roundtrip` (`vb_ipc/src/metrics/tests.rs:298`) already documents this as an allowed field value. | Latent — only reachable if the upstream invariant is broken. |

## Forbidden Patterns (would raise new errors)

| Pattern | Why forbidden |
|---------|---------------|
| `as_conversions` allow on a parent scope (function, module, file) | Violates workspace `as_conversions = "deny"` at the lint-policy layer; the prior bead's local `#[allow]` was the narrowest acceptable scope and the bead is removing it. |
| `as`-cast on `usize` to `f32` or `f64` in `vb_runtime` production source | Same root cause; clippy gate would block landing. |
| `unwrap()` / `expect()` in the replacement expression | Forbidden by `AGENTS.md` "Engineering Rules" ("No `unwrap`, `expect`, `panic`"). The replacement uses `unwrap_or(0)`, which is the allowed escape hatch. |
| Widening the `trace_ring_fill_pct` field type | Out of scope; would break the IPC wire format. |
| Silently flipping the `unwrap_or` fallback to `u32::MAX` | Would corrupt the sentinel intent (see `workflow-model.md` hazard "Ratio saturation by sentinel"). |

## Error Reporting Style

This bead does not introduce or modify any `Error` enum, `Result` return type, `Display` impl, or `From` conversion. The fix is a value-preserving substitution.

## Testing-The-Error Strategy

- Lint gate: `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions` must exit `0`.
- AST gate: `xtask forbidden-scan` must report zero `as`-casts in `vb_runtime` production source (matches `to-fix/wave4/agent-03-black-hat.md:38` evidence request).
- Numeric regression: `cargo test -p vb_runtime --lib trace_ring_fill_pct` must pass all three existing tests.
- Call-site regression: `Runtime::collect_metrics` invoked through `Runtime::new_for_tests_and_benchmarks_only(1, ShardConfig { trace_capacity: 16, ... })` must report `trace_ring_fill_pct == 0.0` for an empty ring, `50.0` for a half-full ring, and `100.0` for a full ring. (Test-writer lane adds these; see `delivery-scope.jsonl` row `r03`.)