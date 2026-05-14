# Verifying Rust With Lean And Gates

Lean is the crown jewel, not the bottleneck. Use it for the smallest pure correctness kernel, then surround the rest of the Rust system with deterministic gates.

## Why Not Lean Everything

Lean proves the specification that was written. It does not prove that the specification captured the whole runtime reality. Keep proofs small enough that their boundary is reviewable.

Use Lean for:

- Pure algorithms and deterministic transforms.
- State-machine legality and impossible states.
- Protocol and authorization rules.
- Arithmetic bounds, overflow freedom, monotonicity, and fixed-point rules.
- Critical data structures such as ring buffers, indexes, heaps, and alloc-free containers.

Do not aim Lean first at:

- I/O, networking, filesystems, databases, clocks, or external services.
- Async runtime scheduling and cancellation semantics.
- FFI and unsafe internals.
- Huge application glue.
- Fast-changing UI or adapter code.

## Current Repo Mapping

This repo should treat `vb_core`, `vb_expr`, and small deterministic pieces of `vb_compile`/`vb_validate` as proof-candidate kernels. Runtime shells such as `vb_runtime`, `vb_storage`, IPC, UI, Makepad, and external adapters stay under Moon, Miri, Kani, fuzzing, mutation, coverage, and manual QA gates.

Future crate splits should follow this shape when the implementation naturally reaches it:

```text
crates/
  vb-core/       # pure, proof-friendly logic
  vb-model/      # executable specs and state machines
  vb-verify/     # Bolero, Kani, Loom, Miri harnesses
  vb-runtime/    # async, IO, networking, DB, OS boundaries
  vb-unsafe/     # isolated unsafe abstractions, only if unavoidable
proofs/
  lean/
```

## Local Verification Modes

```bash
bash scripts/rust-verification-gauntlet.sh fast
bash scripts/rust-verification-gauntlet.sh standard
bash scripts/rust-verification-gauntlet.sh deep
bash scripts/rust-verification-gauntlet.sh proof
bash scripts/rust-verification-gauntlet.sh all
```

Moon exposes the same modes:

```bash
moon run :verify-fast
moon run :verify-standard
moon run :verify-deep
moon run :verify-proof
moon run :verify-all
```

Use this policy:

- `verify-fast`: every local edit loop.
- `verify-standard`: before pushing.
- `verify-deep`: before merge or release.
- `verify-proof`: whenever proof-targeted crates or proof obligations change.
- `verify-all`: release gate.

## Layer Policy

- Pure and critical code: prove the kernel in Lean.
- Bounded invariants: prove with Kani.
- Hostile input, parsers, codecs, and protocols: fuzz with `cargo-fuzz` or Bolero.
- Unsafe or FFI boundaries: run Miri and cargo-careful, and require explicit unsafe contracts.
- Concurrent code: run Loom and Lockbud.
- Test claims: challenge them with cargo-mutants.
- Dependency changes: block on cargo-deny, cargo-vet, cargo-audit, cargo-geiger, and unsafe-budget review.

## Gate Composition

- `fast`: `fmt`, `lint-src`, `check`.
- `standard`: `fast`, `test`, `doc-test`; the repo `test` task already depends on supply-chain checks.
- `deep`: `standard`, Miri, cargo-careful when required, fuzz smoke, Bolero markers, Loom markers, Lockbud for concurrency markers, mutation smoke, coverage.
- `proof`: Kani when required or installed, then `scripts/verify-lean.sh`.
- `all`: `deep` plus `proof`.

Missing proof tools do not silently pass when an obligation requires them. The general repo gate may skip absent Lean/Kani/Bolero/Loom targets only when no source marker or approved proof obligation requires that layer.
