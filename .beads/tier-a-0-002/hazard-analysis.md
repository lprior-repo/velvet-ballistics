# Hazard Analysis — Residue Quarantine CI Gate

bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
phase: 1
state: 3 (rust-contract)
skill: rust-contract
attempt: 1-of-7
updated_at: 2026-06-17T22:00:00.000000+00:00

## 1. Hazard Classification

Hazards are classified into six classes per the rust-contract skill
template:

| Class | Symbol | Definition |
|-------|--------|------------|
| False Positive | FP | A clean source file is flagged as residue. |
| False Negative | FN | A residue-bearing source file is not flagged. |
| Performance | PERF | The gate does not meet the 30-second budget. |
| Regex / Pattern DoS | RE | The pattern matcher can be made to consume excessive time. |
| Configuration Drift | CFG | The gate's policy, allowlist, or moon task drifts from the master. |
| Concurrency / Ordering | CON | The gate's order in the moon pipeline is wrong. |

Each hazard below names the class, the trigger, the impact, the
mitigation, and the owner. The owner is the state that owns the
mitigation; the rust-contract agent commits the mitigation strategy
but does not implement it.

## 2. Hazard H-01: False Positive on Allowlisted Import (FP)

- **Class:** False Positive.
- **Trigger:** A legitimate `use serde_json;` in a dev-dep test
  file (e.g., `crates/vb_core/tests/proptest_serde_roundtrip.rs`)
  is flagged as active residue.
- **Impact:** The gate exits 1 on a clean tree, blocking the moon
  pipeline. The developer must either remove the legitimate use
  (incorrect; it is a test file) or add an allowlist entry (correct).
- **Mitigation:** The allowlist is the legitimate escape hatch. The
  `vb_core/Cargo.toml` line 22 has `# allow-cold-adapter: dev-dep
  test-only, used by serde_json round-trip tests under
  src/action/tests.rs and src/diagnostic/tests_and_verification.rs;
  never linked into runtime`. The State 11 holzman-rust agent must
  populate `scripts/forbid-runtime-fmt.allow` with the corresponding
  entries; the format is documented in `type-contracts.md` §9.1.
- **Owner:** State 11 holzman-rust (populates the allowlist) +
  State 9 test-writer (asserts the allowlist entries are present in
  the fixture tests) + State 14 evidence-packaging (reviews the
  allowlist for drift).
- **Detection:** A failing moon ci run on a clean tree is the
  symptom. The State 13 black-hat-reviewer MUST assert that the
  allowlist is non-empty and that each entry cites `owner=`,
  `reviewed_by=`, `test=`, and `reason=` fields.

## 3. Hazard H-02: False Negative on Variant Import (FN)

- **Class:** False Negative.
- **Trigger:** A forbidden import is introduced in a non-obvious
  form that the gate's pattern table does not match. Examples:
  - `use serde_json::Value;` (matched by `serde_json` substring).
  - `serde_json::from_str(...)` (matched).
  - `extern crate serde_json;` (matched).
  - `extern crate serde_json as sj;` (matched, since the substring
    `serde_json` is in the file).
  - `let v: json::Value = ...;` where `json` is a re-export of
    `serde_json` (NOT matched; the substring `serde_json` is not
    in the file).
  - A `build.rs` file that emits a `serde_json` dependency
    (out of scope; the gate does not scan `build.rs` files because
    they are not in the four hot crate roots under `src/`).
- **Impact:** A forbidden import slips through the gate and lands
  in the runtime core. This is the worst-case failure mode for a
  residue gate.
- **Mitigation:** The pattern table is a *substring* match, not a
  *crate-name* match. The substring `serde_json` matches all known
  variants. The remaining FN risk is for re-exports (a custom
  `json` module that wraps `serde_json`); this risk is bounded
  because such a re-export would itself be a code smell that the
  State 13 black-hat-reviewer would catch on a code review.
- **Owner:** State 13 black-hat-reviewer (manual code review for
  re-exports) + State 14 evidence-packaging (verifies the pattern
  table is up to date with the master).
- **Detection:** A grep for `serde_json|hyper|reqwest|axum|HashMap<String,|tokio::sync::mpsc::unbounded`
  in the four hot crate roots is the canonical cross-check. The
  State 14 evidence-packaging runs this grep as a sanity check
  and reports any divergence from the gate's output.

## 4. Hazard H-03: Performance on Huge Trees (PERF)

- **Class:** Performance.
- **Trigger:** The four hot crate source trees grow to a size
  where a line-by-line walk exceeds the 30-second budget. As of
  2026-06-17, the four hot crates have ~30,000 lines of Rust
  source. A naive single-threaded walk completes in well under
  one second. The 30-second budget is generous.
- **Impact:** The gate's wall-clock time exceeds the moon
  pipeline's per-task budget, causing the pipeline to time out.
- **Mitigation:** The State 11 implementation MUST be linear in
  the total source line count. A line-by-line walk with a BTreeMap
  allowlist lookup is O(N log M) where N is total line count and
  M is allowlist size; with M expected to be < 100, the
  log M factor is negligible.
- **Owner:** State 11 holzman-rust (implementation must hit the
  budget) + State 9 test-writer (the perf-budget test in
  `test-forbid-runtime-fmt.sh` asserts the budget).
- **Detection:** The `test-forbid-runtime-fmt.sh` script runs
  the gate against the real repository and asserts the elapsed
  time is below 30 seconds. A regression is a hard failure of
  the test.

## 5. Hazard H-04: Regex / Pattern DoS (RE)

- **Class:** Regex / Pattern DoS.
- **Trigger:** The scanner uses a regex engine that is vulnerable
  to ReDoS (regular expression denial of service). A pathological
  input file (e.g., a long line of `a` characters) can cause the
  regex to backtrack exponentially.
- **Impact:** The gate's wall-clock time exceeds the budget
  (overlaps with H-03). A malicious developer could intentionally
  add a pathological file to bypass the gate by timing it out.
- **Mitigation:** The scanner MUST use a *substring match* (the
  `str::contains` method), not a regex. Substring matching is
  O(N) in the line length with a small constant factor. The
  pattern table in `type-contracts.md` §6.1 is a list of
  `&'static str` constants, not a regex AST. The contract binds
  this decision: any implementation that uses a regex is a
  contract violation.
- **Owner:** State 11 holzman-rust (uses `str::contains`, not regex)
  + State 13 black-hat-reviewer (asserts the scanner source
  contains no `regex` or `fancy-regex` or `pcre2` imports).
- **Detection:** A grep for `use regex` / `use fancy_regex` /
  `use pcre2` in `scripts/forbid-runtime-fmt.rs` is a hard
  failure of the State 13 review.

## 6. Hazard H-05: Allowlist Bloat (CFG)

- **Class:** Configuration Drift.
- **Trigger:** The allowlist grows to a size where the gate's
  `BTreeMap` lookup is the dominant cost. Or, more importantly,
  the allowlist accumulates entries that mask genuine violations.
- **Impact:** A forbidden import that should fail the gate is
  silently allowlisted; the gate's purpose is defeated.
- **Mitigation:** The allowlist format requires `owner=`,
  `reviewed_by=`, `test=`, and `reason=` fields. The State 14
  evidence-packaging reviews the allowlist on every bead and
  flags entries that:
  - Do not cite a test (the `test=` field is empty).
  - Cite a `reason=` that is shorter than 20 characters
    (heuristic for "this is not a real review").
  - Have a `reviewed_by=` field that matches the `owner=`
    field (self-approval).
  - Have an `owner=` field that is no longer a current
    maintainer (drift from the active maintainer list).
- **Owner:** State 14 evidence-packaging (reviews the allowlist
  per bead) + State 11 holzman-rust (enforces the allowlist
  format at parse time).
- **Detection:** A grep for `|` fields in
  `scripts/forbid-runtime-fmt.allow` is the canonical cross-check.
  The State 14 evidence-packaging produces a
  `truth-serum-report.md` that lists all allowlist entries and
  their `reason=` fields.

## 7. Hazard H-06: Drift Between Master §43 and the Gate's Pattern List (CFG)

- **Class:** Configuration Drift.
- **Trigger:** The master document is amended (§2 or §12 changed)
  but the gate's `ForbiddenImportName` enum is not updated. The
  gate continues to enforce the *old* closed set.
- **Impact:** A new forbidden import introduced after the master
  amendment is not caught by the gate. Conversely, a removed
  forbidden import continues to be enforced, causing false
  positives.
- **Mitigation:** The gate is fail-closed on this hazard: if the
  master is unreadable or its closed sets are inconsistent, the
  gate exits 2 with `GateError::PatternFileMissing`. The
  `ResiduePolicy::from_master` parser is the single point where
  the master is read; any drift surfaces as a parse error.
- **Owner:** State 11 holzman-rust (the parser is the contract
  holder) + State 14 evidence-packaging (asserts the master's
  section numbers and line numbers in the gate's source match
  the master).
- **Detection:** The State 14 evidence-packaging runs a
  cross-check that compares the gate's `ForbiddenImportName`
  enum to the master's §2 and §12 closed sets. Any divergence
  is a `STATUS: REJECTED` finding.

## 8. Hazard H-07: Wrong Order in `moon run :check` Deps (CON)

- **Class:** Concurrency / Ordering.
- **Trigger:** The new gate is added to `.moon/tasks/all.yml::check`
  as a `deps:` entry, but it is ordered *after* the heavier
  compile gates. A forbidden import is therefore caught only
  after a 5-minute `cargo check` run, not before.
- **Impact:** The moon pipeline takes 5+ minutes to fail on a
  one-line violation. The developer experience is poor.
- **Mitigation:** The contract binds the ordering: the new gate
  is added to the `check` task's `deps:` array, ordered before
  the heavier cargo check invocations. The ordering is asserted
  by `test_moon_ci_quarantine_dependency_correctly_ordered`.
- **Owner:** State 11 holzman-rust (places the entry in the
  correct position) + State 9 test-writer (asserts the
  ordering) + State 13 black-hat-reviewer (re-asserts the
  ordering on a code review).
- **Detection:** The third test name
  (`test_moon_ci_quarantine_dependency_correctly_ordered`) is
  the canonical assertion. The test parses `.moon/tasks/all.yml`
  and asserts the gate appears in `check.deps:` before the
  heavier compile gates.

## 9. Hazard H-08: Master Document Not Readable (FN)

- **Class:** False Negative.
- **Trigger:** The scanner's `ResiduePolicy::from_master` parser
  silently succeeds on a missing or empty master document,
  producing an empty `forbidden` list. The gate then passes
  trivially.
- **Impact:** Every forbidden import is bypassed; the gate
  is useless.
- **Mitigation:** The contract binds the parser to be
  fail-closed: a missing master is `GateError::PatternFileMissing`,
  exit 2. A successful parse produces a non-empty `forbidden`
  list with exactly 7 entries. The State 11 holzman-rust
  implementation MUST assert the post-parse invariant
  `policy.forbidden.len() == 7`.
- **Owner:** State 11 holzman-rust (asserts the invariant at
  parse time) + State 9 test-writer (the test
  `test_quarantine_gate_blocks_json_import` exercises the
  invariant by relying on the gate finding the `serde_json`
  import).
- **Detection:** A clean `moon ci` run on a tree that has
  one forbidden import SHOULD fail with exit 1. If it passes
  with exit 0, the parser has silently accepted an empty
  master and the gate is broken.

## 10. Hazard H-09: Allowlist Parse Failure on Comment Lines (FN)

- **Class:** False Negative.
- **Trigger:** The allowlist file has a comment line that
  starts with `#` but the parser treats it as a malformed
  allowlist entry, exiting 2 with `AllowlistParseFailure`.
- **Impact:** A well-formed allowlist with comments is
  rejected; the developer is forced to remove the comments.
- **Mitigation:** The contract binds the allowlist parser to
  skip lines starting with `#` and to skip empty lines. The
  format documented in `type-contracts.md` §9.1 explicitly
  allows comments.
- **Owner:** State 11 holzman-rust (the parser is the contract
  holder) + State 14 evidence-packaging (re-asserts the
  format on a review).
- **Detection:** A manual review of the allowlist file
  confirms the format is comment + entry + comment + entry.

## 11. Hazard H-10: Cold Marker Drift (FN)

- **Class:** False Negative.
- **Trigger:** A new test directory is added to a hot crate
  (e.g., `crates/vb_core/src/benchmark/`) but the cold-marker
  set does not include the new directory name. The gate then
  scans the directory and flags any `serde_json` usage as
  active residue.
- **Impact:** False positives; the gate blocks the moon
  pipeline.
- **Mitigation:** The cold-marker set is closed and copied
  from the sibling `check-hot-cold-forbidden-apis.rs` (lines
  7-23). Adding a new cold marker requires a master amendment
  AND a code change in both scanners. The State 11 holzman-rust
  agent does NOT add new cold markers; if a new test directory
  is needed, it must use one of the existing cold-marker names.
- **Owner:** State 11 holzman-rust (does not add new cold
  markers) + State 14 evidence-packaging (reviews the cold
  marker set for drift).
- **Detection:** A diff between the gate's `ColdMarker` enum
  and the sibling `check-hot-cold-forbidden-apis.rs::COLD_MARKERS`
  constant is a `STATUS: REJECTED` finding.

## 12. Hazard Severity Matrix

| Hazard | Class | Severity | Likelihood | Mitigation Owner |
|--------|-------|----------|------------|------------------|
| H-01 (False Positive on allowlisted) | FP | Medium | High | State 11 + 9 + 14 |
| H-02 (False Negative on variant) | FN | High | Low | State 13 + 14 |
| H-03 (Performance on huge trees) | PERF | Medium | Low | State 11 + 9 |
| H-04 (Regex DoS) | RE | High | Low | State 11 + 13 |
| H-05 (Allowlist bloat) | CFG | High | Medium | State 11 + 14 |
| H-06 (Master drift) | CFG | High | Medium | State 11 + 14 |
| H-07 (Wrong moon order) | CON | Low | Low | State 11 + 9 + 13 |
| H-08 (Master not readable) | FN | Critical | Low | State 11 + 9 |
| H-09 (Allowlist parse failure) | FN | Low | Low | State 11 + 14 |
| H-10 (Cold marker drift) | FN | Medium | Low | State 11 + 14 |

The Critical-severity hazard (H-08) is mitigated by the contract:
the gate fails closed with exit 2 if the master is unreadable.
No implementation can pass the contract tests and silently bypass
this hazard.

## 13. Hazard vs Proof Seed Cross-Reference

| Hazard | Proof seed |
|--------|------------|
| H-01 | RQ-004 (allowlist precedence) |
| H-02 | RQ-002 (forbidden patterns derived from master §43) |
| H-03 | (covered by the 30s perf contract, not a proof seed) |
| H-04 | (covered by the substring-match contract, not a proof seed) |
| H-05 | RQ-004 (allowlist precedence) |
| H-06 | RQ-002 (forbidden patterns derived from master §43) |
| H-07 | (covered by the moon-task contract, not a proof seed) |
| H-08 | RQ-001 (gate is total) |
| H-09 | RQ-005 (gate output is deterministic) |
| H-10 | RQ-001 (gate is total) |

Hazards H-03, H-04, H-07 are covered by *contract* (the contract
binds the substring-match decision, the 30-second budget, and the
moon-task ordering) rather than by *proof seeds*. The proof seeds
cover the *behavior-affecting* hazards.

## 14. Open Hazard Disposition Questions

- The State 11 implementation MAY use `clippy-driver` (heavy
  precedent) instead of `rustc` (light precedent). The contract
  permits both. The decision does not affect hazard disposition
  because both compilers produce the same scanner binary.
- The State 11 implementation MAY choose to scan `Cargo.toml` in
  addition to `.rs` files. The contract does NOT bind this
  decision; the bead description's `/*.rs` suffix is the literal
  scope. A wider scope is permitted but not required.
- The State 11 implementation MAY add timing instrumentation
  behind a `#[cfg(test)]` gate. The contract permits this; the
  released binary MUST NOT have timing instrumentation enabled
  by default.
