# Black Hat Review — vb-7ph Master Document

STATUS: REJECTED

## Executive Verdict

The update is materially better, but it is not yet a mechanical build contract. The author fixed most obvious contradictions, then left poison pills in the core contracts: hot `SlotValue` still carries `Text(Box<str>)`, the engine snippet is not compile-mechanical, the final IR still carries redundant choose forms, and the persistence/ABI sections are too underspecified to implement without guessing.

That is not a contract. That is a polished aspiration document with several sharp edges hidden inside it.

## Phase 1: Contract & Bead Parity

### P0 — Hot value model contradicts the handle-only requirement

- **Lines:** 545-556, 580, 2286
- **Checklist hit:** 3, 5, 9, 10
- **Finding:** `SlotValue` still includes `Text(Box<str>)` at line 552. That is a heap-owned value inside the hot slot model. Line 580 claims `SlotValue` is handle-based, and line 2286 defines completion as handle-based `SlotValue` with `SymbolId`, `ListId`, `ObjectId`, `BlobId`, and finite numbers. The enum contradicts the contract.
- **Why this fails:** Turbo/maxperf says no allocation after admission and hot slots use handles. A boxed string in `SlotValue` is neither numeric nor handle-only. It also reopens the tree-heavy/value-heavy runtime state the stress verdict tried to kill.
- **Required remediation:** Delete `Text(Box<str>)` from hot `SlotValue`. Represent text as `SymbolId`, `BlobId`, or a dedicated interned text handle. Update `type_name`, tests, DoD, and arena contracts accordingly.

### P0 — Core engine snippet is not compile-mechanical

- **Lines:** 545-556, 983-986
- **Checklist hit:** 3, 8, 9
- **Finding:** `drive_deterministic` uses `let value = *frame.read_slot(*result)?;` while `SlotValue` is not `Copy` because it contains `Text(Box<str>)`. This snippet cannot compile as written.
- **Why this fails:** The document claims snippets are contracts for `crates/vb_core/src/` at line 442. A contract snippet that cannot compile is mechanical fraud.
- **Required remediation:** Either make hot `SlotValue` truly `Copy` by removing heap-owned variants, or change the engine result path to clone/move through an explicitly bounded/owned result mechanism. Prefer the former because maxperf demands handle-only values.

### P1 — Canonical spelling exception is wider than the supplied checklist

- **Lines:** 13, 15, 60, 2279
- **Checklist hit:** 1, 10
- **Finding:** The checklist says `velvet-ballistics` is invalid except migration/external artifacts. The document adds an extra exception for unavoidable repository path/file references.
- **Why this matters:** This is probably pragmatic because the repo and file are already named that way, but it is still broader than the stated checklist. The exception must be tightly scoped or agents will abuse it.
- **Required remediation:** Keep only a narrow allowlist: current repository root path and current master filename. Do not leave a generic “where unavoidable” escape hatch.

## Phase 2: Farley Engineering Rigor

### P1 — Function-size rule is too weak for a mechanical build contract

- **Lines:** 99
- **Checklist hit:** 8, 9
- **Finding:** The Holzmann matrix says hot functions “target fewer than 60 lines.” That is a suggestion, not a gate. It also fails the Black Hat hard constraint of 25 lines.
- **Why this fails:** “Target” is not enforceable. A mechanical contract needs a hard limit and an executable check.
- **Required remediation:** Replace with a mandatory limit: hot functions <= 25 logical lines; cold validation phase functions must be decomposed and justified if longer. Add the source-length gate to CI/justfile/Moon tasks.

### P1 — Mandatory functions list names APIs missing from the core snippet contract

- **Lines:** 821-908, 1461, 1468-1469
- **Checklist hit:** 8
- **Finding:** `RunFrame::new`, `RunFrame::read_taint`, and `RunFrame::write_taint` are mandatory at lines 1461 and 1468-1469 but absent from the `frame.rs` contract snippet.
- **Why this fails:** A mechanical implementation handoff should not require implementers to infer constructors or taint access semantics.
- **Required remediation:** Add exact signatures, bounds checks, allocation/admission behavior, and typed errors for these methods.

## Phase 3: NASA-Level Functional Rust / Big 6

### P1 — Choose IR is still muddled

- **Lines:** 691-693, 722-724, 798, 1022-1024
- **Checklist hit:** 4
- **Finding:** The final IR contains `ChooseExpr`, `ChooseSlot`, and `Choose`, where `Choose` is another expression-branch form. This is redundant and invites divergent semantics.
- **Why this fails:** The checklist demanded choose condition representation not be raw `SlotIdx`-only. It does not require three choose variants. Three variants means three chances for generated Rust and IR mode to disagree.
- **Required remediation:** Collapse to two precise forms: expression-branch choose and materialized boolean-slot choose. Delete the ambiguous generic `Choose`, or define it as a deprecated migration-only name outside final IR.

### P1 — Action ABI leaves undefined critical types

- **Lines:** 1213-1265
- **Checklist hit:** 7
- **Finding:** The ABI references `ActionResult`, `ActionOutputReady`, `ActionFailure`, and `ActionError` without defining their fields, error codes, size bounds, taint handling, or binary encoding.
- **Why this fails:** “Exact enough for implementation” means no guessing. This ABI still requires guessing.
- **Required remediation:** Define all referenced ABI types, payload bounds, retry/idempotency semantics, taint propagation, completion encoding, and error variants.

## Phase 4: Ruthless Simplicity & DDD

### P1 — Persistence record envelope is not precise enough

- **Lines:** 1191-1201
- **Checklist hit:** 6
- **Finding:** The binary envelope lists `magic`, `schema_version`, `record_kind`, `payload_len`, `payload` but gives no byte widths, endian rules, magic values per record family, checksum/digest policy, max length enforcement point, or version migration behavior beyond vague words.
- **Why this fails:** Fjall persistence and replay are core safety surfaces. Hand-wavy binary record headers are how corrupt data gets silently normalized.
- **Required remediation:** Specify exact field widths, endian, allowed `record_kind` numeric IDs, max payload length source, digest/checksum verification, decode order, and typed errors.

### P2 — The “MVP IR may start” language weakens finality

- **Lines:** 722, 2220
- **Checklist hit:** 2, 4, 10
- **Finding:** The document allows MVP IR to start with four primitives, then says final IR includes all primitives. That may be intended as phase sequencing, but the wording sits inside the final core contract and gives implementers a loophole.
- **Required remediation:** Move MVP language to implementation phases only. In the final IR section, state only the final required IR contract.

## Phase 5: Bitter Truth

### What passed

- Canonical names are mostly fixed: product/binary/package `velvet-ballastics`, crate/module `velvet_ballastics`, bead rig `velvet-ballastics`, bead database `velvet_ballastics`, version `velvet-ballastics/v1`.
- `manual` direct API and `ipc` binary IPC are both v1 triggers.
- Generated Rust is mandatory for `maxperf`.
- HTTP/JSON are excluded from runtime core.
- `StepBudget::try_take`, `SetConst` typed errors, checked step-state mutation, `ConstValue::to_slot_value`, `FiniteF64`, and missing `CoreError` variants are mostly addressed.
- Hot/cold separation, forbidden hot-path APIs, bounded resource contracts, tooling, phase order, tests, benchmarks, CI/justfile gates, bead breakdown, and 27-point DoD are present in spirit.
- The old contradictions `velvet/v1`, manual-only v1, optional generated Rust, silent Null fallback, `panic = "abort"`, unqualified HashMap ban, and hardcoded `rust-version` requirement are not present as active requirements.

## Required Remediation Before Approval

1. Remove heap-owned `Text(Box<str>)` from hot `SlotValue`; make values genuinely handle-based.
2. Fix the `Finish` snippet so the contract compiles mechanically.
3. Add missing `RunFrame::new`, `read_taint`, and `write_taint` contracts.
4. Tighten the `velvet-ballistics` exception to a narrow allowlist.
5. Replace “hot functions target fewer than 60 lines” with an enforceable hard limit and CI gate.
6. Collapse or precisely define choose IR variants; remove ambiguous `Choose` from final IR if redundant.
7. Fully define all Action ABI referenced types and binary/taint/error semantics.
8. Specify exact binary record envelope layout and decode/recovery rules.
9. Move MVP language out of the final IR contract.

## Brutal Verdict

REJECTED. The update is close, but close is how broken runtimes get built. A mechanical build contract cannot contain a heap string in the hot value enum, a non-compiling engine snippet, undefined ABI types, and a vague persistence envelope. Fix the contract until an implementer can build without interpretation or excuses.
