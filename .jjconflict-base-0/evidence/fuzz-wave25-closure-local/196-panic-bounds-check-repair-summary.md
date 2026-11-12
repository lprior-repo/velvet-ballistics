# panic_bounds_check repair — fuzz_retry_record_attempt

Date: 2026-07-10T12:36:34Z
Workspace: /home/lewis/src/isoloated/velvet-ballistics-w25-fuzz
Git toplevel: /home/lewis/src/isoloated/velvet-ballistics-w25-fuzz
JJ root: /home/lewis/src/isoloated/velvet-ballistics-w25-fuzz

## Scope

Fix `panic_bounds_check` at `fuzz/fuzz_targets/fuzz_retry_record_attempt.rs:120:45`
(old line, pre-fix) discovered by scope-bounded moon ci fuzz-smoke.

Crash artifact: `0a 0a b3 5b` (4 bytes) — exact length range
`len() == 4..8` where the early-return guard (`< 4`) allowed execution to
reach direct indexing of `data[0..8]`.

## Diagnosis

Pre-fix `action_ticket_from_bytes(data: &[u8]) -> Option<ActionTicket>`:

```rust
if data.len() < 4 { return None; }
let run = RunId::new(u64::from_le_bytes([
    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
]));
```

With `data.len() == 4..7`, the indexing array literal evaluates
`data[7]` and panics with `index out of bounds: the len is N but the
index is 7`.

## Fix applied (in working copy state at vrsompwt)

`fuzz/fuzz_targets/fuzz_retry_record_attempt.rs`:

1. Tightened the early-return guard to match the actual 8-byte read:
   `if data.len() < 8 { return None; }` (line 133).
2. Added `read_u64_le_at(data, start)` and `read_u16_le_at(data, start)`
   helpers (lines 42-54) that:
   - bound their start offset via `start.checked_add(N)?`,
   - use `data.get(start..end)` (no panicking indexing), and
   - convert via `<[u8; N]>::try_from(...)`.
3. Replaced the panicking `data[0..8]` indexing with
   `read_u64_le_at(data, 0)?` (line 137).
4. Replaced the panicking `data[10..12]` / `data[12..14]` reads with
   `read_u16_le_at(data, 10).unwrap_or(1)` /
   `read_u16_le_at(data, 12).unwrap_or(1)` (lines 141-142).
5. Replaced direct `data.get(N).copied().unwrap_or(0)` for the byte
   fields (step/seq/action at 8/9/10) — these were already safe.
6. Replaced `let _ = record_retry_attempt(...)` with an explicit
   `match { Ok(_) | Err(_) => {} }` to satisfy the
   `-D clippy::let_underscore_must_use` lint.

`first_chunk::<8>()` semantics are equivalent to `read_u64_le_at(data, 0)`
(returns `Some([u8; 8])` when `data.len() >= 8`, `None` otherwise);
the helper form preserves generality for the `read_u16_le_at` calls at
offsets 10 and 12 without sacrificing Holzman cleanliness.

## Caller audit

`action_ticket_from_bytes` in `fuzz_retry_record_attempt.rs` has a
single caller: the fuzz harness entry point at lines 25-28:

```rust
let ticket = match action_ticket_from_bytes(data) {
    Some(ticket) => ticket,
    None => return,
};
```

The caller already handles `None` correctly. No callers exist outside
the file. Other fuzz targets
(`fuzz_retry_normalize_ticket`, `fuzz_retry_validate_completion`,
`fuzz_retry_postcard_codec`) define their own private copies of
`action_ticket_from_bytes` / `build_run_state` and are unaffected.

## Verification commands run

| # | Command | Status | Evidence file |
|---|---------|--------|---------------|
| 179 | `cargo fmt --check` (root) | exit=0 | `179-root-cargo-fmt-check-after-panic-bounds-check-repair.txt` |
| 180 | `cargo fmt --manifest-path=fuzz/Cargo.toml --check` | exit=0 | `180-fuzz-cargo-fmt-check-after-panic-bounds-check-repair.txt` |
| 181 | `cargo check --workspace --all-targets --all-features` (root) | exit=0 | `181-root-cargo-check-workspace-all-targets-all-features-after-panic-bounds-check-repair.txt` |
| 182 | `cargo check --manifest-path=fuzz/Cargo.toml --all-targets --all-features` | exit=0 | `182-fuzz-cargo-check-all-targets-all-features-after-panic-bounds-check-repair.txt` |
| 183 | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used ...` (root) | exit=0 | `183-root-cargo-clippy-strict-after-panic-bounds-check-repair.txt` |
| 184 | `cargo clippy --manifest-path=fuzz/Cargo.toml --all-targets --all-features -- -D warnings ...` | exit=0 | `184-fuzz-cargo-clippy-strict-all-targets-after-panic-bounds-check-repair.txt` |
| 185 | `cargo fuzz build` | exit=0 | `185-cargo-fuzz-build-after-panic-bounds-check-repair.txt` |
| 186 | `cargo fuzz run fuzz_retry_record_attempt -- -runs=20000 -max_total_time=60` | exit=0, 20000 runs, no crashes | `186-cargo-fuzz-run-fuzz_retry_record_attempt-after-panic-bounds-check-repair.txt` |
| 187 | Direct binary test with original crash artifact `0a 0a b3 5b` | exit=0, executed in 0 ms | `187-direct-binary-test-original-crash-artifact-after-panic-bounds-check-repair.txt` |
| 188 | Edge case inputs n=0..14 | exit=0 for all | `188-edge-case-input-tests-after-panic-bounds-check-repair.txt` |
| 189 | Search for tests touching `action_ticket_from_bytes` / `record_retry_attempt` | (no matches in fuzz/tests/) | `189-search-tests-touching-record-retry-attempt-after-panic-bounds-check-repair.txt` |
| 190 | Search for tests touching `build_run_state` / `retry_policy_from_bytes` | (only private callers) | `190-search-build_run_state-retry_policy_from_bytes-after-panic-bounds-check-repair.txt` |
| 191 | `cargo test --manifest-path=fuzz/Cargo.toml --lib --tests` | exit=0, 6 passed, 0 failed | `191-fuzz-cargo-test-lib-tests-after-panic-bounds-check-repair.txt` |
| 192 | Fjall mandatory + fuzz/Cargo.lock preservation | preserved (fjall 3.1.4) | `192-fjall-and-fuzz-lock-preservation-after-panic-bounds-check-repair.txt` |
| 193 | jj status / log / diff summary | confirmed at vrsompwt | `193-final-jj-checks-and-diff-summary-after-panic-bounds-check-repair.txt` |
| 194 | Final root cargo check (all targets) | exit=0 | `194-final-root-cargo-check-workspace-all-targets-all-features-after-panic-bounds-check-repair.txt` |
| 195 | Workspace identity (root vs fuzz) | root excludes fuzz; fuzz is its own workspace | `195-workspace-identity-check-after-panic-bounds-check-repair.txt` |

## Key fuzz-run numbers

```
#20000	DONE   cov: 331 ft: 338 corp: 11/101b lim: 198 exec/s: 0 rss: 59Mb
Done 20000 runs in 0 second(s)
exit_status=0
```

- 20000 fuzz iterations completed
- 11 corpus files, 101 bytes total corpus size
- 331 coverage units reached (post-init allocation-free hot path)
- 338 feature toggles explored
- 0 crashes / 0 panics / 0 ooms
- exit status 0

## Edge case binary test matrix

n=0..14 against `target/x86_64-unknown-linux-gnu/release/fuzz_retry_record_attempt`:

- n=0,1,2,3: handled by outer harness guard `if data.len() < 4 { return; }` (line 21)
- n=4,5,6,7: previously would panic; now `action_ticket_from_bytes` correctly returns `None` because `data.len() < 8` (line 133), harness takes `None => return` branch
- n=8..14: full `ActionTicket` parsing succeeds via safe `.get()` and `read_*_le_at` helpers

All inputs exit 0 with "Executed /tmp/fuzz_edge_N.bin in 0 ms" (or 1 ms for n=11 due to scheduling).

## Fjall mandatory preservation

- Workspace `Cargo.toml:33`: `fjall = { version = "=3.1.4", default-features = false, features = ["lz4"] }` (exact pin)
- `fuzz/Cargo.lock:541`: `name = "fjall"` `version = "3.1.4"`
- `fuzz/Cargo.lock`: 50765 bytes, MD5 `68a715dfe62efb7c181a9e33040a2560`, intact
- `Cargo.lock` (root): MD5 `b455a9937871cdf0d822e387f259956e`, intact

## Residual risks / blockers

- None observed. All 16 verification gates passed with exit status 0.
- The fix is a minimal, surgical, Holzman-clean repair: tightened
  pre-condition + safe indexing via `.get()` + reusable byte-reading
  helpers.
- The change is contained to a single fuzz target file; no production
  crates are touched.
- jj `@` change id `vrsompwtztlztvsyvkywrrqopmwptlyw` (commit
  `4590b71aeb4ca642ae1f4948a6ba4f1e2a172f0a`) contains the fix in
  working-copy state; not pushed (per task constraints).