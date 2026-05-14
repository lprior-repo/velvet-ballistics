# Black Hat Review — vb-37lc Retry 19

STATUS: APPROVED

Workspace: `/home/lewis/src/vb-37lc`  
Scope: canonical Velvet Ballastics naming scan (`crates/velvet_ballastics/src/naming_scan.rs`, `tests/vb_37lc_canonical_spelling_red.rs`)  
State: 5.5 Black Hat after Red Queen approval and shape repair

## Evidence Run

- `cargo +nightly nextest run --test vb_37lc_canonical_spelling_red` — PASS, 76/76.
- `cargo +nightly clippy --test vb_37lc_canonical_spelling_red -- -D warnings` — PASS.
- `cargo +nightly clippy --manifest-path crates/velvet_ballastics/Cargo.toml -- -D clippy::wildcard_enum_match_arm` — PASS.
- Function-length gate script over `crates/velvet_ballastics/src/naming_scan.rs` — PASS: `max_function_length=25 function=scan_repository lines=382-406`.
- Panic/unsafe scan over `naming_scan.rs` — PASS: no `unwrap(`, `expect(`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, or `unsafe` hits.
- Wildcard catch-all scan over `naming_scan.rs` — PASS: no `_ =>` match arms found.
- Synthetic shortcut/thread-name scan over `naming_scan.rs` — PASS: no thread-name behavior; only `kind_name` identifier noise.
- `git -C /home/lewis/src/vb-37lc diff --quiet -- crates/velvet_ballastics/src/commands_ai_context.rs` — PASS: exit 0, unrelated file is unchanged and legitimately scoped out.

## Phase 1 — Contract & Bead Parity

PASS.

- Canonical spellings exactly match the contract: `velvet-ballastics`, `velvet_ballastics`, `velvet-ballastics/v1` at `naming_scan.rs:5-7` and canonical table construction at `266-276`.
- Contract API exists with the required signatures: `validate_scan_config` (`278-293`), `discover_scan_inputs` (`295-306`), `classify_occurrence` (`308-328`), `scan_file` (`368-380`), `scan_repository` (`382-406`), `render_scan_report` (`408-416`).
- Error taxonomy is represented as `NamingScanError` variants at `255-263`.
- Finding fields satisfy path, line, column, class, remediation at `175-182`; deterministic sorting uses path/line/column/class at `1033-1040`.
- Fowler parity is credible: nextest ran 76 bead-owned tests covering canonical table, invalid spelling, allowlist boundaries, discovery, unreadable input, report writes, and deterministic report order.

## Phase 2 — Farley Engineering Rigor

PASS.

- Hard function limit is now obeyed: measured maximum is 25 lines (`scan_repository`, `382-406`). Previous oversized functions were split.
- No function exceeds 5 parameters in public contract. Private helpers are narrow enough for this mechanical gate; the only 6-parameter private helper is `handle_match` (`710-724`), which is a local shell adapter, not a domain operation. It is ugly, but not a blocker after the successful shape split.
- Functional core is separated enough for a cold quality gate: classification and allowlist logic are pure (`330-657`, `747-858`); filesystem effects are isolated to report write, input read, and discovery (`439-450`, `659-678`, `903-964`).
- Tests assert behavior, not implementation trivia: the suite checks exact returned variants and findings, not private helper calls.

## Phase 3 — Holzman Rust

PASS.

- Illegal outcomes are sum types: `SpellingClass` (`168-173`), `OccurrenceClass` (`199-221`), `NamingScanError` (`255-263`).
- Parse-don't-validate is acceptable for this boundary gate: raw config enters as `RawScanConfig`, validated config exits as `ScanConfig` at `278-293`.
- Broad allowlist variants are explicit and rejected, not silently accepted: `Wildcard`, `PrefixOnly`, `Substring` at `60-81`; rejection at `478-496`.
- No wildcard enum catch-all in the bead-owned source; clippy wildcard enum match arm gate passed.
- Checked arithmetic is used for line/column and scanning offsets (`388-390`, `703-705`, `742-744`, `861-876`). No unchecked indexing/slicing in source; `str::get` is used at `809-818` and `825-827`.

## Phase 4 — Ruthless Simplicity & DDD

PASS.

- No `Option`-based state machine. `Option` is used for ordinary lookup/absence, not workflow state.
- No panic vector in `naming_scan.rs`.
- Domain shape is acceptable for a repository scan: `RepoPath`, `RepoRoot`, `LineNumber`, `ColumnNumber`, `SpellingClass`, `OccurrenceClass`, and `NamingScanError` stop this from degenerating into primitive soup.
- Some string payloads remain (`CanonicalSpellingTable`, `LegacyException`, remediation strings). That is tolerable because they are report/config payloads, not untyped workflow states.

## Phase 5 — Bitter Truth

APPROVED.

The code is boring enough now. The previous bloat was cut down. The behavior is covered. The panic vector is clean. The wildcard survivor is dead. The fake-fixture/thread-name shortcut class of bug is not present.

## Review Focus Verdicts

- Canonical naming scan contract parity: PASS.
- No synthetic shortcuts: PASS. Discovery walks real `fs::read_dir` (`903-909`), uses real entries (`911-948`), and tests the prior shortcut name via `discover_scan_inputs_scans_real_fixture_tree_when_root_name_matches_prior_shortcut`.
- No thread-name behavior: PASS. No thread APIs in bead-owned source.
- No wildcard catch-alls: PASS. Grep found no `_ =>`; clippy wildcard enum gate passed.
- Function shape: PASS. Max function length is 25.
- Legacy allowlist correctness: PASS. Exact rule text and boundary checks are at `790-823`; wildcard/prefix/substring are rejected or ignored when force-injected (`478-496`, tests at `279-323`, `711-725`).
- False positive/negative risk: ACCEPTABLE. Boundary tests for exact repository URL left/right edges pass, embedded migration token tests pass, crate/module and language-version legacy classes pass.
- Real filesystem behavior: PASS. Deterministic discovery sorts inputs at `302-305`; real directory traversal and permission-denied child semantics are at `885-964`.
- Report/error taxonomy: PASS. Typed errors match contract variants at `255-263`; report write failure path is typed at `439-450`; render tests passed.
- `commands_ai_context.rs` scoping: PASS. It is unchanged (`git diff --quiet` exit 0), so Red Queen’s unrelated no-assert issue is not a vb-37lc blocker.

## Brutal Verdict

STATUS: APPROVED

This retry finally stops wasting review time. The contract is met, Red Queen bead-owned evidence is green, hard shape is under the line, and the previous structural blocker is gone. Ship this bead-owned scope; keep the contaminated shared Red Queen blackboard problem out of this bead unless someone explicitly files it as infrastructure work.
