# Workflow Model — vb-tsjnz

- bead_id: `vb-tsjnz`
- workdir: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
- scope: metadata-patch workflow (single-file Cargo.toml edit)
- capture: 2026-07-01

The "workflow" here is the canonical lifetime of the metadata patch through verification and landing. It is smaller than a runtime workflow because there is no business logic in scope — but the **gating states are themselves load-bearing** because turning on workspace lints can flip a previously-green crate to red.

## Legal States

```
   +-----------+      +-----------+      +-----------+
   | Captured  | ---> |  Audited  | ---> |  Prepared |
   +-----------+      +-----------+      +-----------+
                                             |
                                             v
                                       +-----------+
                                       |  Patched  |
                                       +-----------+
                                             |
                                             v
                                       +-------------+
                          +------------+ | BuildVerif- |
                          |            | | ied (cargo  |
                          |            | |  check)     |
                          |            +-------------+
                          |                  |
                          |                  v
                          |            +-----------+
                          |            | LintVerif-|
                          |            | ied (clipy|
                          |            |  -D warn.) |
                          |            +-----------+
                          |                  |
                          |                  v
                          |            +-----------+
                          |            |TestVerif- |
                          |            |ied (tests)|
                          |            +-----------+
                          |                  |
                          v                  v
                   +-----------+      +-----------+
                   |  Failed   | <--- | Released  |
                   +-----------+      +-----------+
```

## State Definitions

| State | Precondition | Hold time | Owner | Exit condition |
| --- | --- | --- | --- | --- |
| `Captured` | Scout has produced `codebase-map.md` + `delivery-scope.jsonl` | instant | explore | Both files exist and pass `jq -c .` |
| `Audited` | `Captured` | seconds | explore (already done by scout) | Drift axes enumerated: (a) literal version; (b) missing `[lints]` |
| `Prepared` | `Audited` | seconds | rust-contract (this bead) | `domain-model.md` + sibling `Cargo.toml` references are committed and form the patch plan |
| `Patched` | `Prepared` | seconds | holzman-rust | `crates/vb_queue_semantics/Cargo.toml` has `version.workspace = true` on line 3 and trailing `[lints]\nworkspace = true` |
| `BuildVerified` | `Patched` | minutes | holzman-rust | `cargo check -p vb_queue_semantics --all-targets` exits 0 |
| `LintVerified` | `BuildVerified` | minutes | holzman-rust | `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` exits 0 |
| `TestVerified` | `LintVerified` | minutes | holzman-rust | Both `cargo test -p workspace_tests --test vb_8ma2_workspace_assertions` and `cargo test -p workspace_tests --test vb_qi37_25_quality_gates` exit 0 |
| `Released` | `TestVerified` | minutes | go-skill landing | PR merged or commit pushed to origin/main; black-hat reviewed |

## Terminal States

- `Released` — green landing.
- `Failed::HardcodedVersionLeak` — post-BuildVerified, grep shows the literal `version = "0.1.0"` somehow still present.
- `Failed::LintFailure` — post-`BuildVerified` lint policy trips on an existing pattern in `vb_queue_semantics/src/lib.rs`. **Abort and re-open contract discussion; do not paper over.**
- `Failed::CargoMetadataFailure` — `cargo metadata` errors out (workspace inheritance does not resolve). Unlikely but covered.
- `Failed::SiblingDivergence` — black-hat review discovers the patch shape diverges from sibling crates (e.g. multi-key `[lints]` table). Reject and redo.
- `Failed::OutOfScopeBleed` — patch touched `crates/vb_queue_semantics/src/lib.rs`, `[dependencies]`, or the `vb-2lu1` exception. Reject and redo.

## Transitions and Guards

T1: `Captured -> Audited`

- **Trigger:** scout reads `crates/vb_queue_semantics/Cargo.toml` and sibling crates.
- **Guard:** scout enumerates drift axes; sibling pattern is documented.
- **Evidence:** `.beads/vb-tsjnz/codebase-map.md`.

T2: `Audited -> Prepared`

- **Trigger:** rust-contract writes the nine artifacts.
- **Guard:** all 9 artifacts exist and JSONL files validate with `jq -c .`.
- **Evidence:** this directory's files.

T3: `Prepared -> Patched`

- **Trigger:** holzman-rust performs the Edit.
- **Guard:** only lines 3 + tail are touched. Diff is two hunks.
- **Evidence:** `jj diff` / `git diff`.

T4: `Patched -> BuildVerified`

- **Trigger:** `cargo check -p vb_queue_semantics --all-targets`.
- **Guard:** exit code 0; no errors. Warnings against `unwrap_used`/`expect_used`/`panic`/etc. count as failures under workspace lint policy.

T5: `BuildVerified -> LintVerified`

- **Trigger:** `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings`.
- **Guard:** exit code 0. Returns non-zero → `Failed::LintFailure`.

T6: `LintVerified -> TestVerified`

- **Trigger:** workspace_tests test runs.
- **Guard:** both `vb_8ma2_workspace_assertions` and `vb_qi37_25_quality_gates` pass.

T7: `TestVerified -> Released`

- **Trigger:** black-hat review accepts.
- **Guard:** diff matches REQ-VBTSJNZ-001..009 (see `contract.md`).

## Idempotence Requirements

- Re-running `cargo check -p vb_queue_semantics --all-targets` after a previously-green state yields identical exit code.
- Re-running `cargo clippy -p vb_queue_semantics --all-targets -- -D warnings` after a previously-green state yields identical exit code.
- The patch is **idempotent under re-application** because the post-patch shape is unique (only one valid `[lints]\nworkspace = true` block fits the file).

## Cancellation / Reversal

Reversal restores the original literal `version = "0.1.0"` and removes the `[lints]` block — exactly the inverse of the patch. Cancellation is permitted at any state; only `Released` requires an in-tree revert commit.

## Retries

| State | Retryable? | Retry strategy |
| --- | --- | --- |
| `Patched` | yes, holzman-rust re-edits | Re-edit until cargo check is green |
| `BuildVerified` | yes | Address build error, re-run cargo check |
| `LintVerified` | yes | Address clippy deny, re-run clippy. **Refuse** to alter workspace lints; the only legitimate fix is in `vb_queue_semantics` source, and **that source edit is the next bead, not this one**. This bead would `Failed::LintFailure → escalate`. |
| `TestVerified` | yes | Address test failure; in this bead, the test surface is unrelated to the patch, so failure implies an environmental issue |
| `Released` | via new commit (revert) | not a retry, a regression |

## Concurrency / Asynchrony

N/A — synchronous patch flow.

## Hazards (Summary; full list in `hazard-analysis.md`)

H-LINT-FORWARD: enabling workspace lints is **forward-applying**; the build step is the actual enforcement. The patch compiles only if the source is already lint-clean. Scout reports it is, holzman-rust MUST re-verify.

H-DRIFT-BACK: a future bump of `[workspace.package].version` automatically propagates to `vb_queue_semantics`. Out of scope here.

H-MANIFEST-DRIFT: a future reintroduction of `version = "..."` reintroduces the P1 drift. Out of scope.
