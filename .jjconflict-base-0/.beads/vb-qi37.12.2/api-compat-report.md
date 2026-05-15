# API Compatibility Report — vb-qi37.12.2 State 11 Rerun

STATUS: PASS

Tool: `cargo-semver-checks 0.47.0`.

## Command

`env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo semver-checks -p vb_runtime --baseline-rev HEAD`

Result: PASS.

```text
Checking vb_runtime v0.1.0 -> v0.1.0 (no change; assume minor)
Checked 196 checks: 196 pass, 56 skip
Summary no semver update required
Finished vb_runtime
```

API-COMPAT-001: PASS; public `ResumeError` shape is semver-compatible with the local `HEAD` baseline.
