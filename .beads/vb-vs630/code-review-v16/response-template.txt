# Black Hat Reviewer Response Template

## Header
```
**Bead**: <bead_id>  
**State**: <state_number>  
**Reviewer**: black-hat-reviewer  
**Source checkout**: <path>  
**Attempt**: <attempt_number>
```

## Gate Result
**STATUS: APPROVED** | **STATUS: REJECTED**

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| ... | ✅/❌ | ... |

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status |
|----------|-------|-------|--------|
| ... | ... | 25 | ✅/❌ |

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status |
|------|--------|
| Zero `unsafe` | ✅/❌ |
| Zero `.unwrap()`/`.expect()` | ✅/❌ |
| Zero `panic!`/`todo!`/`dbg!` | ✅/❌ |
| Checked arithmetic | ✅/❌ |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status |
|-------|--------|
| No Option-based state machines | ✅/❌ |
| CUPID compliant | ✅/❌ |
| No clever abstractions | ✅/❌ |

---

## PHASE 5: The Bitter Truth

[Clinical assessment of code quality]

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| ... | CRITICAL/HIGH/MEDIUM/LOW | ... | fixed/open |

### [FINDING-XXX]: [Title]

**Location**: `file:line`

**Problem**: [Clear description]

**Evidence**: [Specific code snippet or command output]

**Required Fix**: [Specific action]

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p <package>` | ✅/❌ | ... |
| `cargo clippy -p <package>` | ✅/❌ | ... |
| `cargo fmt --check` | ✅/❌ | ... |

---

## Verdict

**STATUS: APPROVED** or **STATUS: REJECTED**

### Summary
[2-3 sentence clinical summary]

---

## Required Repair Actions (if REJECTED)

1. [CRITICAL/HIGH/MEDIUM]: [Action required]
2. ...
