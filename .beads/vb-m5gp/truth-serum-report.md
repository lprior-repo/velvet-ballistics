# Truth Serum Report: vb-m5gp

STATUS: APPROVED

## Startup Doctrine Cited

- Read `/home/lewis/.claude/skills/truth-serum/SKILL.md`: mandates direct command evidence, no delegated proof, zero runtime panic surface, no laundered evidence.
- Read `/home/lewis/.agents/skills/truth-serum/SKILL.md`: same content observed; `.agents` wins on conflict.

## Execution Evidence

Commands were executed directly in `/home/lewis/src/go-skill-vb-m5gp` unless noted.

```text
$ pwd -P && rtk git status --short
/home/lewis/src/go-skill-vb-m5gp
fatal: not a git repository (or any parent up to mount point /)
Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).
```

Decision: not a Git worktree; this is a jj workspace as recorded by State 1. Follow-up status used `jj status`.

```text
$ jj status
Working copy changes:
... scoped State 1-12 bead artifacts, vb_compile split files, split contract test, and source-length script ...
Working copy  (@) : lwuzwvry 738fa064 go-skill vb-m5gp state workspace
Parent commit (@-): ysnxntql cc80fac3 fix: correct schema_version in .cue contract files to 1.0.0
```

```text
$ python - <<'PY'
... validate required artifact presence and STATUS lines ...
PY
missing= []
STATE.md: bytes=45693 STATUS: <none>
contract.md: bytes=6354 STATUS: <none>
traceability-matrix.jsonl: bytes=6872 STATUS: <none>
api-compat-report.md: bytes=735 STATUS: PASS
formal-verification-report.md: bytes=5463 STATUS: APPROVED
verification-ledger.jsonl: bytes=5426 STATUS: <none>
machine-gate-report.md: bytes=1519 STATUS: PASS
test-report.md: bytes=907 STATUS: PASS
source-length-report.md: bytes=869 STATUS: PASS
static-scan-report.md: bytes=1102 STATUS: PASS
kani-report.md: bytes=389 STATUS: PASS
miri-report.md: bytes=524 STATUS: DEFERRED_GLOBAL
regression-diff.md: bytes=766 STATUS: PASS
black-hat-review.md: bytes=4041 STATUS: APPROVED
```

```text
$ python - <<'PY'
... parse JSONL ledgers and reject required non-pass rows ...
PY
traceability-matrix.jsonl: rows=24
proof-obligations.jsonl: rows=15
proof-obligations.planned.jsonl: rows=20
verification-ledger.jsonl: rows=15
delivery-scope.jsonl: rows=1
required_non_pass= []
deferred_global= ['MIRI-001']
```

```text
$ bash scripts/check-source-length.sh
DEFERRED_GLOBAL: crates/vb_compile/src/expression_bytecode.rs has 2242 physical lines (limit <300)
DEFERRED_GLOBAL: crates/vb_compile/src/expression.rs has 881 physical lines (limit <300)
DEFERRED_GLOBAL: crates/vb_compile/src/references.rs has 342 physical lines (limit <300)
DEFERRED_GLOBAL: crates/vb_compile/src/schema.rs has 729 physical lines (limit <300)
DEFERRED_GLOBAL: crates/vb_compile/src/type_taint.rs has 511 physical lines (limit <300)
```

```text
$ python - <<'PY'
... recursive bead-local lib.rs/mod_compile_* source length count ...
PY
bead_local_files= 28
max= (286, 'crates/vb_compile/src/mod_compile_errors/collection.rs')
oversized= []
```

```text
$ python - <<'PY'
... targeted forbidden dependency/exposure scan ...
PY
errors_to_validation=0 []
validation_to_core=0 []
validation_to_lowering=0 []
include_bodies=0 []
compile_core_impl=0 []
pub_mod_compile=0 []
```

```text
$ cargo +nightly test -p velvet-ballastics-workspace-tests --test vb_m5gp_compile_split_contract mod_compile_dependency_edges_remain_acyclic_and_diagnostic_leaf -- --exact
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.53s
     Running tests/vb_m5gp_compile_split_contract.rs (target/debug/deps/vb_m5gp_compile_split_contract-6d6e03c9db8f6086)

running 1 test
test mod_compile_dependency_edges_remain_acyclic_and_diagnostic_leaf ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s
```

```text
$ cargo +nightly check -p vb_compile --all-targets --all-features
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
```

```text
$ cargo +nightly clippy -p vb_compile --lib --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use -D clippy::await_holding_lock
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Blocking waiting for file lock on package cache
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.13s
```

## Empathetic User Review

- The evidence trail is dense but navigable: reports are colocated under `.beads/vb-m5gp/` and use clear `STATUS:` headers where human-facing decisions matter.
- The only confusing point is environment/tooling: this workspace is jj-managed, so `git status` fails. That is recorded as a non-blocking context fact, not hidden.

## Skeptical QA Review

- Required evidence is present and machine-readable enough to audit: JSONL ledgers parse, no required verification row is non-pass, and all named high-risk artifacts exist.
- No laundered approval accepted blindly: State 13 reran targeted source-length, forbidden-edge, API compile, and strict clippy checks directly.
- `MIRI-001` is not laundered: it remains explicitly `DEFERRED_GLOBAL`, `required:false`, with a concrete missing rust-src blocker and compensating `moon ci` lane evidence.
- Source-length debt is not hidden: five unrelated legacy files remain `DEFERRED_GLOBAL`, while bead-local `lib.rs` and `mod_compile_*` files are below 300 lines.

## Mandated Improvements

- For this bead: none blocking.
- Follow-up outside this bead: repair local nightly rust-src/Miri path and decompose unrelated oversized legacy files tracked as `DEFERRED_GLOBAL` debt.
