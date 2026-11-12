# Research Notes: vb-70z3b — Boundary: YAML/JSON/HTTP Free Runtime Core

## Scope
Mechanically verify that `vb_runtime` and `vb_core` crates contain no runtime YAML, JSON, or HTTP dependencies or imports.

## Methodology
1. Searched all `.rs` files in `crates/vb_runtime/src/` and `crates/vb_core/src/` for `yaml`, `json`, `http` patterns.
2. Searched all `Cargo.toml` files for runtime dependencies on `serde_yaml`, `serde_json`, `hyper`, `reqwest`, `actix`, `http`, `json-rpc`.
3. Used `cargo tree` to verify runtime dependency chains.
4. Inspected module inclusion gating (`#[cfg(test)]`, `#[cfg(kani)]`).

## Findings

### vb_runtime — CLEAN
**Cargo.toml dependencies:** `blake3`, `chrono`, `crossbeam-queue`, `indexmap`, `rtrb`, `thiserror`, `vb_core`, `vb_storage`, `postcard`, `serde`
- No YAML, JSON, or HTTP crates.
- `serde` is the serialization framework only (not JSON-specific).
- `postcard` is a binary serialization format (not JSON).
- **No `use` or `extern` statements** for any JSON/YAML/HTTP crates in production code.

**YAML references found (all gated):**
- `kani_engine_yaml_admission` — gated by `#[cfg(all(kani, feature = "kani-engine-yaml-admission"))]` (line 64-65 of lib.rs)
- `yaml_e2e_admission_matrix` — gated by `#[cfg(all(kani, feature = "kani-yaml-e2e-admission-matrix"))]` (line 92-93 of lib.rs)
- These are Kani verification harnesses, not production code.

**cargo tree verification:**
- `serde_yaml` — not in dependency tree (package not found)
- `serde_json` — not in dependency tree
- `http`/`hyper`/`reqwest` — not in dependency tree

### vb_core — CLEAN
**Cargo.toml dependencies:** `bytes`, `chrono`, `indexmap`, `serde`, `thiserror`
- No YAML, JSON, or HTTP crates in runtime dependencies.
- `serde_json` appears only in `[dev-dependencies]` (test-only).

**`serde_json` usage (test-gated only):**
- `crates/vb_core/src/diagnostic/tests_and_verification.rs` lines 217-230: uses `serde_json::to_string` and `serde_json::from_str`
- Module included via `#[cfg(test)]` at `diagnostic.rs:2092-2094`
- Only compiled during `cargo test`, not in production builds.

**`kani/kani_serde_roundtrip.rs`** — contains helper functions with `json_str` parameter names but performs manual string parsing (no serde_json import).

**cargo tree verification:**
- `serde_yaml` — not in dependency tree
- `serde_json` — appears only as `[dev-dependencies]` on `vb_core`
- `http`/`hyper`/`reqwest` — not in dependency tree

### workspace_tests (separate crate — NOT in scope)
- Has `serde_yaml` and `serde_json` in `[dependencies]` (runtime)
- This is a test harness crate, not part of the runtime core.
- Per AGENTS.md: `crates/workspace_tests/` contains cross-crate integration tests and benchmarks.
- Does not affect vb-70z3b's scope.

## Conclusion
**No violations found.** The runtime core (`vb_runtime` + `vb_core`) is mechanically free of YAML, JSON (as runtime dependency), and HTTP dependencies. All YAML/JSON references are either:
1. Kani verification harnesses (gated by `cfg(kani)`)
2. Test-only code (gated by `#[cfg(test)]`)
3. Dev-dependencies only

## Raw Evidence Commands

```bash
# serde_yaml in vb_runtime
cargo tree -p vb_runtime -e features -i serde_yaml
# Result: error: package ID specification `serde_yaml` did not match any packages

# serde_yaml in vb_core
cargo tree -p vb_core -e features -i serde_yaml
# Result: error: package ID specification `serde_yaml` did not match any packages

# serde_json in vb_runtime
cargo tree -p vb_runtime -e features -i serde_json
# Result: warning: nothing to print.

# serde_json in vb_core (dev-only)
cargo tree -p vb_core -e features -i serde_json
# Result: shows [dev-dependencies] only, no runtime path

# http/hyper/reqwest in vb_runtime
cargo tree -p vb_runtime -e features -i http
# Result: error: package ID specification `http` did not match any packages

# http/hyper/reqwest in vb_core
cargo tree -p vb_core -e features -i http
# Result: error: package ID specification `http` did not match any packages

# Production use/extern search
rg -n '^use (serde_json|serde_yaml|yaml_rust|hyper|reqwest|actix|jsonrpc)' --type rust crates/vb_core/src/ crates/vb_runtime/src/
# Result: no matches

rg -n '^extern (crate )?(serde_json|serde_yaml|yaml_rust|hyper|reqwest|actix|jsonrpc)' --type rust crates/vb_core/src/ crates/vb_runtime/src/
# Result: no matches
```
