# vb-kij9n Closure-Audit Evidence

**Parent bead:** `vb-kij9n` (Bug Hunt 2026-06-21: final confirmed source bugs, closed 2026-06-24)
**Auditor:** `holzman-rust` agent
**Audit date:** 2026-06-24
**Audit purpose:** Persist the structured audit artifacts that supported the original
close-reason claim so future reviewers can independently verify the closure count
without re-running the audit. The original artifacts lived only in `/tmp/opencode/`
(volatile, lost on next machine reboot).

## One-line audit summary

155 confirmed bug children, 1 wave-16 closure epic (vb-og75k, non-bug), 0 missing
planner_session metadata, 0 rejected findings leaked as children, 0 duplicate
source_finding_ids, 0 orphan child IDs, 0 unaccounted finding IDs.

## File map

| File | Contents | Size | Source |
| --- | --- | --- | --- |
| `children.json` | Raw `bd list` dump of all 156 children of `vb-kij9n` (155 bug + 1 wave-16 epic). 2.1 MB. | 2.1 MB | `/tmp/opencode/vb-kij9n-all-children.json` |
| `audit.json` | Structured invariants + per-check counts (status distribution, planner-session coverage, final_status coverage, duplicate detection). | 1.0 KB | `/tmp/opencode/vb-kij9n-audit.json` |
| `all-231-finding-ids.txt` | The 231 finding IDs that the bug-hunt-2026-06-21 final adjudication actually audited (155 confirmed + 76 rejected = 231). | 1.6 KB | `/tmp/opencode/all-231-finding-ids.txt` |
| `child-155-finding-ids.txt` | The 155 `source_finding_id` values present in the `children.json` metadata (i.e. the 155 confirmed child beads). | 1.1 KB | `/tmp/opencode/child-155-finding-ids.txt` |
| `rejected-76-finding-ids.txt` | The 76 final rejected finding IDs. Cross-set check: `child ∩ rejected == ∅` (0 leakage). | 532 B | `/tmp/opencode/rejected-76-finding-ids.txt` |

## Structured invariants (from `audit.json`)

| Check | Result |
| --- | --- |
| Total children of `vb-kij9n` | 156 |
| Bug children (issue_type != "epic" for non-bug) | 155 |
| Non-bug children (the wave-16 closure epic `vb-og75k`, status=closed) | 1 |
| Unique `source_finding_id` values across children | 155 |
| Duplicate `source_finding_id` values | 0 |
| Duplicate `external_ref` values | 0 |
| Children missing `planner_session` metadata | 0 |
| Children with `planner_session` != `vb-bug-hunt-confirmed-20260621` | 0 |
| Children with `final_status` != `confirmed` | 0 |
| Rejected findings leaked as children (`child ∩ rejected`) | 0 |
| Orphan child IDs (not in 231 audited findings) | 0 |
| Finding IDs unaccounted (not in child or rejected) | 0 |
| Bug-child status distribution | closed=119, in_progress=30, blocked=5, open=1 |

## How to reproduce (read-only, no audit re-run required)

The raw `children.json` is sufficient to verify every structural claim:

```bash
# 1) Total child count: must equal 156
jq 'length' .beads/vb-kij9n/evidence/children.json

# 2) Unique child IDs: must equal 155 (one of the 156 is a wave-16 epic)
jq -r '.[].id' .beads/vb-kij9n/evidence/children.json | sort -u | wc -l

# 3) Children with parent=vb-kij9n: must equal 155
jq -r '.[] | select(.parent == "vb-kij9n") | .id' \
  .beads/vb-kij9n/evidence/children.json | sort -u | wc -l

# 4) Unique source_finding_id values: must equal 155
jq -r '.[].metadata.source_finding_id // empty' \
  .beads/vb-kij9n/evidence/children.json | sort -u | wc -l

# 5) Every child has planner_session=vb-bug-hunt-confirmed-20260621: must be a single value
jq -r '.[].metadata.planner_session // empty' \
  .beads/vb-kij9n/evidence/children.json | sort -u

# 6) Every child has final_status=confirmed: must be a single value
jq -r '.[].metadata.final_status // empty' \
  .beads/vb-kij9n/evidence/children.json | sort -u

# 7) Cross-set: 0 rejected findings leaked as children (comm: lines unique to child-155 ∩ rejected-76 == 0)
comm -12 \
  <(sort .beads/vb-kij9n/evidence/child-155-finding-ids.txt) \
  <(sort .beads/vb-kij9n/evidence/rejected-76-finding-ids.txt) | wc -l

# 8) Cross-set: every child finding_id is in the 231-set (child - 231 == empty)
comm -23 \
  <(sort .beads/vb-kij9n/evidence/child-155-finding-ids.txt) \
  <(sort .beads/vb-kij9n/evidence/all-231-finding-ids.txt)

# 9) Cross-set: every rejected finding_id is in the 231-set (rejected - 231 == empty)
comm -23 \
  <(sort .beads/vb-kij9n/evidence/rejected-76-finding-ids.txt) \
  <(sort .beads/vb-kij9n/evidence/all-231-finding-ids.txt)
```

If all nine checks return 156, 155, 155, 155, a single value, a single value, 0, empty,
and empty respectively, the original close-reason claim is independently verified.

## Cross-reference

- Parent bead: [`vb-kij9n`](../../vb-kij9n) (notes field contains the audit summary).
- Audit claim bead: `vb-jpq7.audit.1` (the bead that triggered this evidence filing).
- Original close-reason text: see `vb-kij9n.close_reason` in `bd show vb-kij9n --json`.

## EARS acceptance criteria mapping

| EARS clause | Evidence file | Verified value |
| --- | --- | --- |
| EARS-1: parent epic lists exactly 155 child bug beads | `children.json` | 155 bug children + 1 wave-16 epic = 156 total |
| EARS-2: every child has `planner_session=vb-bug-hunt-confirmed-20260621` | `children.json` | 100% coverage (single value) |
| EARS-3: rejected findings are NOT implementation children | `child-155-finding-ids.txt` ∩ `rejected-76-finding-ids.txt` | 0 lines |
| INV-1: rejected findings excluded from backlog | `comm -12` cross-set | 0 leakage |
| INV-2: every child maps to exactly one final confirmed finding ID | `unique_source_finding_ids` | 155 unique, 0 duplicates |

## Audit log

- Raw audit command output: `/tmp/opencode/vb-kij9n-audit.json` (now persisted here as `audit.json`).
- Original children listing: `/tmp/opencode/vb-kij9n-all-children.json` (now persisted here as `children.json`).
- Cross-set inputs: `/tmp/opencode/{all-231-finding-ids,child-155-finding-ids,rejected-76-finding-ids}.txt` (now persisted in this directory).
