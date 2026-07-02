# Machine Gate Report — vb-qi37.12.2 State 11 Rerun

STATUS: APPROVED

Workspace: `/home/lewis/src/vb-qi37-12-2`; forbidden checkout `/home/lewis/src/Velvet-ballistics` not used.

Skill files read/cited: `/home/lewis/.claude/skills/formal-verifier/SKILL.md` lines 14, 21-24, 100-114 and `/home/lewis/.agents/skills/formal-verifier/SKILL.md` lines 14, 21-24, 100-114; `.agents` wins and matches.

## Gates

- Artifact / JSONL gate: PASS — isolated workspace; required artifacts present; `contract-verification-review.md` has `STATUS: APPROVED`; `jq` parsed required JSONL.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo fmt --check`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --test vb_qi37_12_2_resume_error_propagation --all-features`: PASS, 7 passed / 0 failed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_runtime --lib is_resumable`: PASS, 2 passed / 0 failed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo clippy -p vb_runtime --lib --tests --all-features -- -D warnings`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo clippy -p vb_runtime --lib --bins --examples --all-features -- -D warnings`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo semver-checks -p vb_runtime --baseline-rev HEAD`: PASS, 196 pass / 56 skip.
- Scoped `cargo-mutants` resume/is_resumable lane: PASS, 6 mutants tested / 5 caught / 1 unviable / 0 missed; exact `RuntimeState::is_resumable` filter also PASS, 2 caught / 0 missed.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo check -p vb_ipc --all-features`: PASS.
- `env TMPDIR=/home/lewis/src/vb-qi37-12-2/.tmp RUSTC_WRAPPER= cargo test -p vb_ipc --all-features`: PASS, 407 passed / 0 failed; doc tests 0.

## Decision

STATUS: APPROVED
