# Baseline Report — vb-qi37.2.1

## Bead

| Field | Value |
|-------|-------|
| bead_id | vb-qi37.2.1 |
| title | runtime: Define aggregate resource budget model |
| parent_commit | 336dbd58bfb5d17ccacb75dfb2713e17ac002e46 |
| baseline_date | 2026-05-13 |

## Source Checkout Baseline

| Metric | Value |
|--------|-------|
| tracked_files | 2359 |
| changed_files (pre-edit) | 10 |

## Pre-Edit Diff Summary

```
.beads/vb-h8h0/contract.md             | 25 ++++++++++++++
.beads/vb-h8h0/proof-obligations.jsonl |  4 +++
.beads/vb-o313/contract.md             | 22 +++++++++++++
.beads/vb-o313/proof-obligations.jsonl |  2 ++
.beads/vb-wgew/contract.md             | 59 ++++++++++++++++++++++++++++++++++
.beads/vb-wgew/proof-obligations.jsonl |  4 +++
.beads/vb-wzg5/contract.md             | 24 ++++++++++++++
.beads/vb-wzg5/proof-obligations.jsonl |  3 ++
fuzz/Cargo.toml                        |  1 +
fuzz/src/lib.rs                        | 54 +++++++------------------------
10 files changed, 156 insertions(+), 42 deletions(-)
```

## Baseline Description

Pre-edit baseline captured at commit `336dbd58`. The checkout contains 2359 tracked source files. There are 10 files with pre-existing uncommitted changes across bead contracts and the fuzz crate. These changes are unrelated to vb-qi37.2.1 and represent prior bead work in the checkout.

## Isolated Workspace

| Field | Value |
|-------|-------|
| workspace | /home/lewis/src/vb-qi37-2-1 |
| isolation_verified | true |
| inside_source_checkout | false |
