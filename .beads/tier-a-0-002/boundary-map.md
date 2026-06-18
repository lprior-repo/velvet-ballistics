# Boundary Map — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.00+00:00 (note: ISO-8601 micros)
updated_at_iso: 2026-06-17T22:00:00.000000+00:00

## 1. Boundary Topology

The gate lives at a single shell-and-rustc boundary. The boundary
spans four planes:

```
                +----------------------+
                |  moon ci pipeline    |  (orchestration)
                +----------+-----------+
                           |
                           | moon run :forbid-runtime-fmt
                           v
                +----------------------+
                |  bash wrapper        |  (imperative shell)
                |  forbid-runtime-fmt.sh|
                +----+-----------+-----+
                     |           |
       rustc build   |           |   run
                     v           v
        +------------+--+    +---+----------------+
        | rustc compiler|    | scanner binary     |  (compiled with
        +---------------+    | forbid-runtime-fmt |   rustc --edition=2024)
                             +---------+----------+
                                       |
                                       | read
                                       v
                             +---------+----------+
                             | crates/{hot}/src/  |  (file system boundary)
                             | scripts/forbid-... |  (config boundary)
                             | velvet-ballistics- |  (master contract boundary)
                             |   MASTER.md        |
                             +--------------------+
```

The gate has exactly **one inbound boundary** (the moon pipeline)
and **one outbound boundary** (the file system). The bash wrapper
is the only piece that crosses both; the scanner is the only piece
that touches the file system.

## 2. Inbound Boundary: moon ci

### 2.1 Inputs from moon

The moon task (whether `forbid-runtime-fmt.yml` or a new entry in
`all.yml`) receives:

- `command: 'bash scripts/forbid-runtime-fmt.sh'` — the only command.
- `inputs:` — a list of globs that moon uses for cache invalidation:
  - `scripts/forbid-runtime-fmt.sh` (the bash wrapper).
  - `scripts/forbid-runtime-fmt.rs` (the scanner source).
  - `scripts/forbid-runtime-fmt.allow` (the allowlist).
  - `fixtures/forbid-runtime-fmt/**/*` (the test fixtures).
  - `crates/vb_core/src/**/*` (one glob per hot crate).
  - `crates/vb_runtime/src/**/*`.
  - `crates/vb_storage/src/**/*`.
  - `crates/vb_ipc/src/**/*`.
  - `.moon/tasks/all.yml` (the task graph).
- `options.runInCI: true` — the gate runs in moon ci.
- `deps:` — the gate is a dependency of `:check`, ordered before
  the heavier cargo check invocations.

### 2.2 Outputs to moon

- **Exit code:** 0 (pass), 1 (active residue), 2 (contract
  violation), or 64 (invalid invocation from bash pre-flight).
- **stdout:** the summary line on a passing run; nothing on a
  failing run.
- **stderr:** the residue-match lines and the summary line on a
  failing run; the `GateError:*` line on a contract violation; the
  bash error message on a pre-flight failure.

### 2.3 Trust boundary

The moon pipeline is trusted. The gate does not validate moon's
inputs; if moon invokes the gate with the wrong arguments, the
bash pre-flight catches it (cwd check) and exits 64. The gate does
not accept any runtime arguments from moon beyond what the bash
wrapper's pre-flight permits.

## 3. Outbound Boundary: File System

### 3.1 Read paths

The scanner reads:

- `velvet-ballistics-MASTER.md` (the master contract).
- `crates/vb_core/src/**/*.rs` (one of four hot crate roots).
- `crates/vb_runtime/src/**/*.rs`.
- `crates/vb_storage/src/**/*.rs`.
- `crates/vb_ipc/src/**/*.rs`.
- `scripts/forbid-runtime-fmt.allow` (the allowlist; may not exist;
  the loader creates an empty `AllowlistRef` in that case).

The scanner never reads:

- `Cargo.toml` (out of scope per OQ-004).
- `Cargo.lock` (out of scope per OQ-006).
- Any file outside the four hot crate roots.
- Any file in a cold path (silently skipped).
- Any file in `target/`, `.git/`, `.beads/`, `.moon/`, `docs/`,
  `tests/`, `fixtures/`, `kani/`, `verification/`, `proofs/`,
  `xtask/`, `fuzz/`, `transcripts/`, `to-fix/`, `supply-chain/`,
  `arch-drift-reports/`, `reference/`, `specs/`, `design/`,
  `contracts/`, `schemas/`.

### 3.2 Write paths

The scanner does not write any file. The bash wrapper writes only
to `target/gate-tools/forbid-runtime-fmt` (the compiled binary).
Moon writes its own cache and lock files; the gate does not
interfere with moon's state.

### 3.3 Trust boundary

The file system is untrusted. The gate's safety properties rely on
the file system being:

- **Readable** — every file the gate needs to read is readable.
  A read failure on a hot path is `GateError::GlobUnreadable`.
- **Unmodified mid-scan** — the scanner assumes the file system
  does not change during one scan. A change mid-scan is
  undefined behavior; the bash wrapper's `flock` on
  `target/moon-locks/source-mutation.lock` (matching the
  `lint-src` precedent) serializes the scan with respect to
  other moon tasks that mutate the source tree.
- **Not hostile** — the scanner does not defend against malicious
  input. If a hot crate's source file contains adversarial text
  designed to confuse the scanner, the scanner will report
  whatever it matches. The risk is bounded by the closed set
  of forbidden imports: a malicious file can only *trigger*
  false positives, not bypass the gate.

## 4. Internal Boundary: bash wrapper to scanner

### 4.1 Pre-flight

The bash wrapper runs `pwd -P` and asserts that the current working
directory contains both `Cargo.toml` and `crates/`. This is the
only validation the wrapper performs before invoking the scanner.

### 4.2 Build

The wrapper runs `rustc --edition=2024 scripts/forbid-runtime-fmt.rs
-o target/gate-tools/forbid-runtime-fmt`. The `target/` directory is
created if it does not exist. A `rustc` failure is caught by
`set -e` and exits 2 with a bash error message.

### 4.3 Run

The wrapper invokes `target/gate-tools/forbid-runtime-fmt` with no
arguments. The scanner's `main` function returns a
`Result<GateDecision, GateError>`; the wrapper propagates the
`Result`'s exit code to moon.

### 4.4 Trust boundary

The scanner binary is trusted. The bash wrapper does not validate
the scanner's output; it trusts the scanner to emit the canonical
format. A scanner bug that emits a non-canonical format is a
contract violation that the State 13 black-hat-reviewer must
catch.

## 5. Boundary Diagram (Logical)

```
+-------------------+        +-------------------+
|   Moon Pipeline   | -----> |  bash wrapper     |
|   (trusted)       | <----- |  (untrusted-by-   |
|                   |        |   contract)       |
+-------------------+        +---------+---------+
                                        |
                                        | rustc build
                                        v
                              +---------+---------+
                              |  Scanner binary   |
                              |  (trusted)        |
                              +---------+---------+
                                        |
                                        | file I/O
                                        v
                              +---------+---------+
                              |  File System      |
                              |  (untrusted)      |
                              +-------------------+
```

The "trusted" labels are about contract enforcement, not about
security. The gate is a *static* scanner; it does not defend against
hostile input. The trust labels indicate which components the
contract treats as "the contract holder" (and is therefore the
State 13 black-hat-reviewer's target) versus which components are
treated as "the contract consumer" (and is therefore the State 14
evidence-packaging's target).

## 6. External Dependencies

The gate depends on the following external tools. The State 11
implementation does not control these tools' behavior; the contract
treats them as black boxes.

| Tool | Version requirement | Used for |
|------|----------------------|----------|
| `bash` | 5.x (the `set -euo pipefail` flag is mandatory) | shell wrapper |
| `rustc` | pinned to nightly-2026-04-28 (per `rust-toolchain.toml`) | compile the scanner |
| `pwd` | coreutils | pre-flight cwd check |
| `dirname` | coreutils | resolve script paths |
| `mkdir` | coreutils | create `target/gate-tools/` |
| `stat` / `find` | coreutils | (optional) cold-path classifier may use either |
| `grep` / `rg` | (optional) | not used by the scanner; the scanner reads files directly with `std::fs` |
| `moon` | v2 (per `.moon/`) | invoke the gate as a task |

The scanner does NOT depend on `grep` or `rg`; it reads files
directly with `std::fs::read_to_string`. The dependency on
external text-search tools would be a contract violation because
it would couple the gate's behavior to a tool whose output format
is not the gate's contract.

## 7. Async / Concurrency Boundary

The gate has no async boundary. The scanner is single-threaded;
the bash wrapper is single-process. There is no `tokio`, no
`async`, no thread spawn. The State 11 holzman-rust implementation
MUST NOT introduce async I/O; the contract treats async as a
`forbid(unsafe_code)`-style hard-deny.

## 8. Unsafe Boundary

The scanner has `#![forbid(unsafe_code)]`. The bash wrapper has
no equivalent pragma (bash does not have unsafe in the Rust sense),
but the wrapper is forbidden from using `eval` or any other
dynamic-construct mechanism. The contract treats `eval` as a
contract violation.

## 9. Network Boundary

The gate has no network boundary. The scanner does not perform
network I/O. The bash wrapper does not invoke `curl`, `wget`, or
`git fetch`. The contract treats any network call as a contract
violation.

## 10. Time Boundary

The gate has no time boundary. The scanner does not call
`std::time::Instant::now` or similar. The bash wrapper records
the wall-clock time only for the perf-budget assertion in
`test-forbid-runtime-fmt.sh`; the gate itself is time-agnostic.

The State 11 implementation MAY add timing instrumentation behind
a `#[cfg(test)]` gate for the perf-budget test, but the released
binary MUST NOT have timing instrumentation enabled by default.

## 11. Storage Boundary

The gate has no storage boundary. The scanner does not read or
write to a database, a key-value store, or a file outside the
four hot crate roots and the allowlist file. The contract treats
any storage call as a contract violation.

## 12. FFI Boundary

The gate has no FFI boundary. The scanner does not call into C,
C++, or any other language. The contract treats any FFI call as a
contract violation.

## 13. Boundary Summary

| Boundary | Direction | Trust | Hardness |
|----------|-----------|-------|----------|
| moon ci → bash wrapper | inbound | trusted | contract |
| bash wrapper → scanner | internal | trusted (post-build) | contract |
| scanner → file system | outbound | untrusted | contract |
| scanner → master document | outbound (read) | untrusted | contract |
| scanner → allowlist | outbound (read) | untrusted | contract |
| bash wrapper → target/ | internal | trusted | filesystem |
| scanner → stdout | outbound | trusted | contract (stderr format) |
| scanner → stderr | outbound | trusted | contract (stderr format) |
| scanner → process exit code | outbound | trusted | contract (exit code table) |

The boundary map is a contract; the State 13 black-hat-reviewer
MUST reject any implementation that adds a boundary not listed
above (e.g., a network call, an FFI call, a database call).

## 14. Boundary Map → Master Document

| Master section | Boundary element |
|----------------|------------------|
| §2 "No JSON in the runtime core" | scanner reads `crates/<hot>/**/*.rs` and matches `serde_json` substring |
| §2 "No HTTP in the runtime core" | scanner matches `hyper` / `reqwest` / `axum` substrings |
| §2 "No `HashMap<String, Value>` runtime state" | scanner matches `HashMap<String,` substring |
| §12 `serde_json` | same as §2 line 99 |
| §12 `HashMap<String, _>` | same as §2 line 102 |
| §12 `unbounded channel creation` | scanner matches `tokio::sync::mpsc::unbounded` substring |
| §43 triggers 7-10 | protected by the union of all 7 `ForbiddenImportName` variants |
| §44.6 "JSON and HTTP are absent from `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`" | the four `QuarantinedCrate` entries are the file-system read roots |
| §78 "scripts/forbid-runtime-fmt.sh exit 0" | the bash wrapper is the contract holder for exit 0 on a clean tree |

## 15. Boundary Map → External Tools

| External tool | Boundary element |
|---------------|------------------|
| `rustc` | compiles the scanner; the scanner's behavior is the contract, not `rustc`'s |
| `bash` | runs the wrapper; the wrapper's behavior is the contract, not `bash`'s |
| `moon` | invokes the gate; the gate's behavior is the contract, not `moon`'s |
| file system | provides source files; the source file contents are the contract input |
| `velvet-ballistics-MASTER.md` | provides the policy; the policy is parsed by the scanner at runtime |
