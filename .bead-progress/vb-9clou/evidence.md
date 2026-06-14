# vb-9clou — cold-adapter-isolation-audit

## Bead

`vb-9clou` — implement a deterministic scanner that audits the four
runtime-core boundary crates (`vb_core`, `vb_runtime`, `vb_storage`,
`vb_ipc`) for ACTIVE HTTP / JSON / YAML / adapter-only dependencies
or `use` / `extern crate` imports.

Master quote (verbatim, `velvet-ballistics-MASTER.md:62`):

> "HTTP and JSON are excluded from the v1 runtime core. Any future
> adapter must be a separate cold-path adapter crate and must not
> enter `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`."

## Overlap Audit (vb-4c1k)

Bead `vb-4c1k` (closed, P1) titled "Add runtime-core adapter
contamination guard" provides PARTIAL coverage. The active artifacts
are:

- `xtask/src/dependency_boundary.rs:41` — `assert_runtime_dependency_boundary`
  in-process helper used by the `restate_adapter_contamination_guard.rs`
  proptest. It constructs **synthetic** `WorkspaceManifest::from_edges(...)`
  data, never reads the real `crates/vb_core/Cargo.toml` or any
  `src/**/*.rs` file. The proptest feeds hand-built `(crate, dep)`
  pairs into the helper. This proves the helper's logic but does NOT
  prove the real boundary crates are clean.
- `scripts/check-workspace-assertions.rs:38` — declares
  `FORBIDDEN_RUNTIME_FORMAT_DEPENDENCIES = ["serde_json", "saphyr",
  "saphyr-parser", "serde-saphyr"]` and only checks `[dependencies] /
  [dev-dependencies] / [build-dependencies]` of the 4 boundary crates
  in its `check_forbidden_dependencies` (lines 346–371). It does NOT
  scan `src/**/*.rs` for `use <forbidden>` imports.

**Verdict:** PARTIAL COVERAGE. `vb-4c1k` covers:
- ✅ manifest-level dependency checking (4 boundary crates, same set of
  forbidden tokens for JSON/YAML),
- ❌ source-level `use`/`extern crate` scanning of the 4 boundary crates,
- ❌ file:line diagnostics,
- ❌ per-line `# allow-cold-adapter:` allowlist markers,
- ❌ HTTP client tokens (`reqwest`, `hyper`, `axum`, `ureq`, `attohttpc`,
  `isahc`) — `vb-4c1k` only forbids HTTP *server* tokens via the
  proptest helper, which checks the same 4 crates for `reqwest`, `hyper`,
  etc., but again on synthetic data.

This bead delivers the complement: deterministic, real-file scanning
of both manifests and source files with file:line diagnostics, a
broader HTTP/JSON/YAML forbidden-token set, and a per-line allowlist
mechanism.

## Files Created

1. `scripts/check-cold-adapter-isolation.rs`     (Rust scanner, Holzman compliant)
2. `scripts/check-cold-adapter-isolation.sh`     (bash wrapper, builds + runs the .rs)
3. `scripts/test-check-cold-adapter-isolation.sh` (self-test, 5 cases: positive, negative serde_json, negative http, allowlisted, real repo)
4. `fixtures/cold-adapter-isolation/positive.rs`           (clean use of `vb_core::types::...`; scanner must report 0 active)
5. `fixtures/cold-adapter-isolation/negative.rs`           (`use serde_json::{json, Value};`; scanner must report 1 active)
6. `fixtures/cold-adapter-isolation/negative_http.rs`      (`use hyper::body::Body; use reqwest::Client;`; scanner must report 2 active)
7. `fixtures/cold-adapter-isolation/negative_allowlisted.rs` (`# allow-cold-adapter: ...` then `use reqwest::Client;`; scanner must report 0 active + 1 allowlisted)

## Scanner Contract

Scope (hard-coded; not configurable from CLI):
- `crates/vb_core/Cargo.toml`
- `crates/vb_runtime/Cargo.toml`
- `crates/vb_storage/Cargo.toml`
- `crates/vb_ipc/Cargo.toml`
- `crates/vb_core/src/**/*.rs`
- `crates/vb_runtime/src/**/*.rs`
- `crates/vb_storage/src/**/*.rs`
- `crates/vb_ipc/src/**/*.rs`

Forbidden tokens (whole-word, hyphen-/underscore-safe):
- `serde_json`, `saphyr`, `saphyr-parser`, `serde-saphyr`,
- `reqwest`, `hyper`, `axum`, `ureq`, `attohttpc`, `isahc`.

Cargo.toml checks: only `[dependencies]` and `[dev-dependencies]`
tables. The left-hand side of `<name> = ...` (the dep name, including
`<name>.<attr>` workspace-inheritance form) is matched against the
forbidden set.

Source checks: any line whose first non-`//` token, after trimming, is
`use <forbidden>` or `extern crate <forbidden>`. Bare path references
like `serde_json::to_string(...)` are intentionally NOT flagged: the
spec is "scan for `use`/`extern crate` imports", not "any reference".
This keeps test files that exercise `serde_json` for round-trip checks
out of scope; the master contract is about the *import surface*, not
runtime references.

Per-line allowlist: a line containing `# allow-cold-adapter: <reason>`
or `// allow-cold-adapter: <reason>` suppresses the NEXT non-blank line.
The suppressed line is reported as `allowlisted:` and never causes a
failure.

Output (stderr):
- `<rel>:<lineno>: COLD-ADAPTER: <crate>: <context>: <line>` (active)
- `<rel>:<lineno>: allowlisted: <reason>: <line>` (suppressed)
- Final: `summary: active=N allowlisted=M files_scanned=K`.

Exit 0 if `active == 0`, exit 1 otherwise.

Self-skip: scanner's own files (`check-cold-adapter-isolation.rs`,
`check-cold-adapter-isolation.sh`, `test-check-cold-adapter-isolation.sh`)
and the standard skip dirs (`target`, `node_modules`, `.bead-progress`,
`.evidence`, plus any dotfile-prefixed directory).

## Holzman Compliance

```text
$ rtk rg -n '\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|dbg!|unsafe[^_]' \
    scripts/check-cold-adapter-isolation.rs
(no matches; exit 1 = nothing found)

$ rtk rg -n 'as isize|as usize' scripts/check-cold-adapter-isolation.rs
(no matches; no unchecked casts)

$ rustc --edition=2024 scripts/check-cold-adapter-isolation.rs \
    -o target/gate-tools/check-cold-adapter-isolation
(exit 0; clean compile)
```

## Raw Command Evidence

### 1. `ls -la` of the 8 deliverable files

```text
$ ls -la scripts/check-cold-adapter-isolation.{sh,rs} \
        scripts/test-check-cold-adapter-isolation.sh \
        fixtures/cold-adapter-isolation/ \
        .bead-progress/vb-9clou/evidence.md
-rw-r--r-- 1 lewis lewis  641 ...  scripts/check-cold-adapter-isolation.rs
-rwxr-xr-x 1 lewis lewis 1.7K ...  scripts/check-cold-adapter-isolation.sh
-rwxr-xr-x 1 lewis lewis 4.6K ...  scripts/test-check-cold-adapter-isolation.sh

fixtures/cold-adapter-isolation/:
-rw-r--r-- 1 lewis lewis  998 ...  fixtures/cold-adapter-isolation/negative.rs
-rw-r--r-- 1 lewis lewis  614 ...  fixtures/cold-adapter-isolation/negative_allowlisted.rs
-rw-r--r-- 1 lewis lewis  575 ...  fixtures/cold-adapter-isolation/negative_http.rs
-rw-r--r-- 1 lewis lewis  879 ...  fixtures/cold-adapter-isolation/positive.rs

.bead-progress/vb-9clou/:
-rw-r--r-- 1 lewis lewis ... ...  .bead-progress/vb-9clou/evidence.md
```

### 2. Self-test (`test-check-cold-adapter-isolation.sh`)

```text
$ bash scripts/test-check-cold-adapter-isolation.sh
[1/5] positive fixture must PASS (exit 0, no active findings)
  ok: exit 0
  ok: summary reports active=0
[2/5] negative serde_json fixture must FAIL (exit 1, file:line finding)
  ok: exit 1 with file:line finding
  ok: token reported as COLD-ADAPTER: serde_json
[3/5] negative http fixture must FAIL (exit 1, hyper+reqwest findings)
  ok: exit 1
  ok: hyper + reqwest both reported
[4/5] negative allowlisted fixture must PASS (exit 0, allowlisted=1)
  ok: exit 0
  ok: allowlisted marker consumes the violation
[5/5] real repository scan must complete and emit a summary line
  ok: summary line emitted
  ok: real-repo exit code: 1 (see evidence.md for context)
self-test PASSED
$ echo $?
0
```

### 3. Real repository scan (corrected expectation)

The user-supplied verification step asserted "real repo is clean
(exit 0)". The actual repository is NOT clean: `crates/vb_core/Cargo.toml:20`
contains `serde_json.workspace = true` under `[dev-dependencies]`. The
scanner correctly detects this active violation and exits 1.

```text
$ bash scripts/check-cold-adapter-isolation.sh
crates/vb_core/Cargo.toml:20: COLD-ADAPTER: serde_json: forbidden dependency in [dependencies]/[dev-dependencies]: serde_json.workspace = true
summary: active=1 allowlisted=0 files_scanned=823
$ echo $?
1
```

This is **evidence the scanner is working**, not a false positive:
the master contract is being violated. Closing this bead without
either removing the dev-dep or applying an explicit
`# allow-cold-adapter: <reason>` marker would be a
"false green" (see bead Section 3 — Inversions: "False green: a Moon
task or script exists but is smoke-only, no-op, disabled, or not in
CI"). The dev-dep predates this scanner and is used by tests in
`crates/vb_core/src/diagnostic/tests_and_verification.rs` and
`crates/vb_core/src/action/tests.rs` for serde_json round-trip
verification. A future bead (or follow-up in this same bead) must
either:

1. add a `# allow-cold-adapter: legacy serde_json round-trip tests
   in src/action/tests.rs and src/diagnostic/tests_and_verification.rs;
   contract was previously not enforced` marker on the line above
   the violation, OR
2. move the test fixtures out of `crates/vb_core/src/**/*.rs` into a
   cold-path test crate (e.g. `vb_runtime/cold_adapter_isolation_tests.rs`),
   OR
3. explicitly carve out a `[dev-dependencies] serde_json = ...` exception
   via the workspace-level `check-workspace-assertions.rs` fence.

### 4. Positive fixture

```text
$ bash scripts/check-cold-adapter-isolation.sh \
    fixtures/cold-adapter-isolation/positive.rs
summary: active=0 allowlisted=0 files_scanned=1
$ echo $?
0
```

### 5. Negative serde_json fixture

```text
$ bash scripts/check-cold-adapter-isolation.sh \
    fixtures/cold-adapter-isolation/negative.rs
fixtures/cold-adapter-isolation/negative.rs:14: COLD-ADAPTER: serde_json: forbidden `use`/`extern crate` import in source: use serde_json::{json, Value};
summary: active=1 allowlisted=0 files_scanned=1
$ echo $?
1
```

### 6. Negative http fixture (hyper + reqwest)

```text
$ bash scripts/check-cold-adapter-isolation.sh \
    fixtures/cold-adapter-isolation/negative_http.rs
fixtures/cold-adapter-isolation/negative_http.rs:14: COLD-ADAPTER: hyper: forbidden `use`/`extern crate` import in source: use hyper::body::Body;
fixtures/cold-adapter-isolation/negative_http.rs:15: COLD-ADAPTER: reqwest: forbidden `use`/`extern crate` import in source: use reqwest::Client;
summary: active=2 allowlisted=0 files_scanned=1
$ echo $?
1
```

### 7. Allowlisted fixture (reqwest suppressed by marker)

```text
$ bash scripts/check-cold-adapter-isolation.sh \
    fixtures/cold-adapter-isolation/negative_allowlisted.rs
fixtures/cold-adapter-isolation/negative_allowlisted.rs:21: allowlisted: historical example - reqwest shape is here to prove the allowlist path: use reqwest::Client;
summary: active=0 allowlisted=1 files_scanned=1
$ echo $?
0
```

### 8. Holzman compliance rg

```text
$ rtk rg -n '\.unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|dbg!|unsafe[^_]' \
    scripts/check-cold-adapter-isolation.rs
(no matches; scanner is Holzman compliant)
```

## Honest Discrepancy With the Bead Request

The bead's verification step asserted:

> `bash scripts/check-cold-adapter-isolation.sh    # must exit 0 (real repo is clean)`

This assertion is **factually wrong** with respect to the current
repository state. The real repository has a `serde_json` workspace
dev-dependency on `vb_core` (line 20 of `crates/vb_core/Cargo.toml`)
which the scanner correctly flags. The scanner is doing its job.

The three honest next steps are listed in §3 above. Closing this bead
should record that the scanner is delivered AND that the real-repo
status is "1 active violation", not "clean". The scanner, self-test,
and fixtures are the deliverables; the violation must be remediated in
a separate bead so this one does not become a "false green" inversion.
