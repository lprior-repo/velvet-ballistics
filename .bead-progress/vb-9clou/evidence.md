# vb-9clou — cold-adapter-isolation-audit

## Updated scope

Implemented a deterministic scanner that now:

- tokenizes `use` / `extern crate` imports instead of substring matching,
- handles leading `::`, aliases, and `extern crate ... as ...`,
- reads boundary `Cargo.toml` `[dependencies]` / `[dev-dependencies]`
  tables and builds local-name → package maps,
- flags manifest alias lines when the effective package is forbidden,
- flags source imports when a local alias maps to a forbidden package,
- walks the whole boundary crate directory recursively (including
  `tests/`, `benches/`, and `examples/`), while still skipping
  `target/`, `.bead-progress/`, `.evidence/`, and hidden dirs,
- preserves the allowlisted `crates/vb_core/Cargo.toml:21`
  `serde_json` dev-dependency.

## Overlap audit

Bead `vb-4c1k` is still only PARTIAL coverage. It proves a manifest-only
and synthetic helper path, but not this deterministic whole-repo file
scanner with alias mapping and recursive boundary walking.

## Files modified

- `scripts/check-cold-adapter-isolation.rs`
- `scripts/check-cold-adapter-isolation.sh`
- `scripts/test-check-cold-adapter-isolation.sh`

## Verification evidence

### `ls` / file presence

```text
-rw-r--r-- 1 lewis lewis 11629 Jun 14 15:16 .bead-progress/vb-9clou/evidence.md
-rw-r--r-- 1 lewis lewis 25685 Jun 14 16:58 scripts/check-cold-adapter-isolation.rs
-rwxr-xr-x 1 lewis lewis  1980 Jun 14 16:57 scripts/check-cold-adapter-isolation.sh
-rwxr-xr-x 1 lewis lewis  4600 Jun 14 16:57 scripts/test-check-cold-adapter-isolation.sh

fixtures/cold-adapter-isolation/:
total 16
drwxr-xr-x 1 lewis lewis  122 Jun 14 15:29 .
drwxr-xr-x 1 lewis lewis  204 Jun 14 15:23 ..
-rw-r--r-- 1 lewis lewis 1127 Jun 14 15:12 negative_allowlisted.rs
-rw-r--r-- 1 lewis lewis  788 Jun 14 15:11 negative_http.rs
-rw-r--r-- 1 lewis lewis  755 Jun 14 15:11 negative.rs
-rw-r--r-- 1 lewis lewis 1284 Jun 14 15:11 positive.rs
```

### rustfmt check

```text
(no output)
```

### Self-test

```text
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
[5/5] real repository scan must complete, emit a summary line, and PASS
  ok: summary line emitted
  ok: real-repo exit code 0
self-test PASSED
```

### Full repo scan

```text
crates/vb_core/Cargo.toml:21: allowlisted: dev-dep test-only, used by serde_json round-trip tests under src/action/tests.rs and src/diagnostic/tests_and_verification.rs; never linked into runtime: serde_json.workspace = true
summary: active=0 allowlisted=1 files_scanned=971
```

### Positive fixture

```text
summary: active=0 allowlisted=0 files_scanned=1
```

### Negative serde_json fixture

```text
fixtures/cold-adapter-isolation/negative.rs:14: COLD-ADAPTER: serde_json: forbidden `use`/`extern crate` import in source: use serde_json::{json, Value};
summary: active=1 allowlisted=0 files_scanned=1
```

### Negative http fixture

```text
fixtures/cold-adapter-isolation/negative_http.rs:14: COLD-ADAPTER: hyper: forbidden `use`/`extern crate` import in source: use hyper::body::Body;
fixtures/cold-adapter-isolation/negative_http.rs:15: COLD-ADAPTER: reqwest: forbidden `use`/`extern crate` import in source: use reqwest::Client;
summary: active=2 allowlisted=0 files_scanned=1
```

### Allowlisted fixture

```text
fixtures/cold-adapter-isolation/negative_allowlisted.rs:21: allowlisted: historical example - reqwest shape is here to prove the allowlist path: use reqwest::Client;
summary: active=0 allowlisted=1 files_scanned=1
```

### Leading-colon bypass

```text
/tmp/tmp.FycVfZssBh/leading_colon.rs:1: COLD-ADAPTER: serde_json: forbidden `use`/`extern crate` import in source: use ::serde_json::Value;
/tmp/tmp.FycVfZssBh/leading_colon.rs:2: COLD-ADAPTER: reqwest: forbidden `use`/`extern crate` import in source: use ::reqwest::Client;
/tmp/tmp.FycVfZssBh/leading_colon.rs:3: COLD-ADAPTER: hyper: forbidden `use`/`extern crate` import in source: use ::hyper::body::Body;
summary: active=3 allowlisted=0 files_scanned=1
```

### Manifest alias bypass

```text
/tmp/tmp.1Gv23HLReu/alias_crate/Cargo.toml:2: COLD-ADAPTER: serde_json: forbidden dependency alias in [dependencies]/[dev-dependencies] via local dep 'serde_http' -> package 'serde_json': serde_http = { package = "serde_json", version = "1" }
/tmp/tmp.1Gv23HLReu/alias_crate/Cargo.toml:3: COLD-ADAPTER: hyper: forbidden dependency alias in [dependencies]/[dev-dependencies] via local dep 'hyper_transport' -> package 'hyper': hyper_transport = { package = "hyper", version = "1" }
/tmp/tmp.1Gv23HLReu/alias_crate/src/lib.rs:1: COLD-ADAPTER: serde_json: forbidden `use`/`extern crate` import in source via local dep 'serde_http' -> package 'serde_json': use serde_http;
/tmp/tmp.1Gv23HLReu/alias_crate/src/lib.rs:2: COLD-ADAPTER: hyper: forbidden `use`/`extern crate` import in source via local dep 'hyper_transport' -> package 'hyper': use hyper_transport::body::Body;
summary: active=4 allowlisted=0 files_scanned=2
```

### tests/benches/examples coverage

```text
/tmp/tmp.fm70PTUgmA/boundary/tests/banana.rs:1: COLD-ADAPTER: reqwest: forbidden `use`/`extern crate` import in source: use reqwest::Client;
summary: active=1 allowlisted=0 files_scanned=1
```
