bead_id: vb-kyyf
bead_title: bdd: Cross-run determinism and reproducibility acceptance scenarios
phase: 1
updated_at: 2026-05-18T01:39:21.506799+00:00
attempt: 1

# Baseline Report

Purpose: pre-edit baseline marker for go-skill State 11 regression classification. No production/test/proof edits have been made in this isolated workspace before this report.

source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/femdation-vb-kyyf-bdd
workspace_parent_revision: main
canonical_gate_policy: moon ci is the repository release gate; State 11 must run scoped/canonical gates and compare against this pre-edit marker plus any known global blockers.

## Pre-edit workspace status

### pwd -P

exit=0 cwd=/home/lewis/src/femdation-vb-kyyf-bdd

```text
/home/lewis/src/femdation-vb-kyyf-bdd

```

### jj status

exit=0 cwd=/home/lewis/src/femdation-vb-kyyf-bdd

```text
The working copy has no changes.
Working copy  (@) : vkyrrmvt c2b12ef9 (empty) femdation workspace vb-kyyf
Parent commit (@-): qqltoylp 597ec09c main | chore(beads): keep server metadata mode

```

### jj workspace list

exit=0 cwd=/home/lewis/src/femdation-vb-kyyf-bdd

```text
default: qmxouvll 18cb982a (no description set)
femdation-vb-0253-1: vtlntzqx 8b6b1729 (conflict) femdation workspace vb-0253.1
femdation-vb-0253-2: nqwrlqxo b151ddf0 (conflict) femdation workspace vb-0253.2
femdation-vb-0253-5: vktpzslz 44d370da (conflict) femdation workspace vb-0253.5
femdation-vb-0253-7: nyputrkz 91f2bb4c (no description set)
femdation-vb-9ihz: ymwtsltt d8bbfe76 (no description set)
femdation-vb-core-accepted-artifact-format: qszwoswz e9814d1e (conflict) femdation workspace vb-core-accepted-artifact-format
femdation-vb-core-bd-reliability: vruokzwm e54d4233 (conflict) (empty) (no description set)
femdation-vb-core-ipc-loom-property: rqllmkpu e5df7b83 (conflict) femdation workspace vb-core-ipc-loom-property
femdation-vb-core-lower-control-primitives: yslzpvnk bd741280 (conflict) femdation workspace vb-core-lower-control-primitives
femdation-vb-core-lower-coverage-matrix: olsnkpsq 016f489e (conflict) femdation workspace vb-core-lower-coverage-matrix
femdation-vb-core-proof-gate-inputs: ktywlvuw a6cfac4b (conflict) femdation workspace vb-core-proof-gate-inputs
femdation-vb-core-strict-ack-ordering: vvpnxwvm c8518227 (conflict) femdation workspace vb-core-strict-ack-ordering
femdation-vb-core-trigger-contract: otvlrumk b4d09819 femdation workspace vb-core-trigger-contract
femdation-vb-d12k: vluwkywr 902e3a9b (no description set)
femdation-vb-kyyf-bdd: vkyrrmvt c2b12ef9 (empty) femdation workspace vb-kyyf
femdation-vb-qi37-2-4: ptyusxzy 8ec6f49b (empty) (no description set)
femdation-vb-qi37-5-3: lwmwmvzn 7d2b53d1 (conflict) femdation workspace vb-qi37.5.3
femdation-vb-qi37-6: krozluvs 8643d6e8 femdation workspace vb-qi37.6
femdation-vb-qk69: yvotzknk 67bf899f (no description set)
femdation-vb-ssei-bdd: nypmpkkz d7cd7c08 (empty) femdation workspace vb-ssei
femdation-vb-targets: ootwppzz a2e4ca76 (empty) (no description set)
femdation-vb-vt2f-bdd: rmmnwzkt 97befdde (empty) femdation workspace vb-vt2f
femdation-vb-xtg4: vzxyzzkr ab3ff85b (no description set)
go-skill-p0-vb-ahfl: mpnopplk b531be55 (empty) go-skill-p0-vb-ahfl | vb-ahfl: COMPLETE States 13-15 landing - truth-serum PASS, evidence-packaging APPROVED, final-evidence-decision APPROVED, jj push SUCCESS, bd close SUCCESS, cleanup COMPLETE
go-skill-p0-vb-core-atomic-admission: ltsqmout 90543c09 (empty) (no description set)
go-skill-p0-vb-core-cli-accepted-path: xklumvvy 32f18b37 (empty) (no description set)
go-skill-p0-vb-core-ipc-sync-evidence: pkqkktvw b76defad (empty) (no description set)
go-skill-p0-vb-core-storage-artifact-store: xoykrxut 0e74d69b (empty) (no description set)
go-skill-p0-vb-core-yaml-e2e-chain: wotuqpny f813386f (empty) (no description set)
go-skill-p0-vb-engine-yaml: xtusnpwn 62a1833e (empty) (no description set)
go-skill-p0-vb-f04l: xqnqpwpp ca4bf05d (empty) (no description set)
go-skill-p0-vb-qi37-1: sqsrzuou 8cdaeb7e (empty) (no description set)
go-skill-p0-vb-qi37-1-6: zonxtvpl 2dbe1bbe (empty) (no description set)
go-skill-p0-vb-qi37-12: tkwxyuky 6eb9aa08 (empty) (no description set)
go-skill-p0-vb-qi37-12-2: qtuxovmm 7d699843 (empty) (no description set)
go-skill-p0-vb-qi37-12-4: txnwuwns ce9c6673 (empty) (no description set)
go-skill-p0-vb-qi37-2: vlwwrztm 5fa6d2c6 (empty) (no description set)
go-skill-p0-vb-qi37-2-4: nwmmvttr 54d6b448 (no description set)
go-skill-p0-vb-qi37-2-5: tqmkklzz 798f9137 (empty) (no description set)
go-skill-p0-vb-qi37-4: xnknpsmy e31b18ee (empty) (no description set)
go-skill-p0-vb-qi37-4-2: kknyvzps b00a2e28 (empty) (no description set)
go-skill-p0-vb-qi37-5: kqoytzvw 0250b5b4 (empty) (no description set)
go-skill-p0-vb-qi37-5-3: oxpmqklz 5b4ebbe2 (empty) (no description set)
go-skill-p0-vb-qi37-6: oszsryxx 4c10c3e4 (empty) (no description set)
go-skill-p0-vb-xkli: orpwwvzn 41c81369 (empty) (no description set)
go-skill-vb-2b4g: qyzyuwwv 6cef1d1a (empty) post-landing clean workspace
go-skill-vb-8ma2: wuormynl aff191dc (empty) (no description set)
go-skill-vb-8ma2-sub2: vnsrznnp 2493413e (no description set)
go-skill-vb-c3k9-owner: xsqspzrv 86e26d9d (empty) (no description set)
go-skill-vb-c3k9-sub1: tsslrrqw cec40425 (no description set)
go-skill-vb-c3k9-sub3: rrrpmlnq 006c4936 (no description set)
go-skill-vb-c3k9-sub4: slxqrswv ee8a5689 (no description set)
go-skill-vb-gvmt: qtloqpus 42fd9cb0 (empty) (no description set)
go-skill-vb-hjvq-sub7: kulvywkm 76bdca97 (no description set)
go-skill-vb-hxm0-sub6: nmmrnnry 743fb26c (empty) (no description set)
go-skill-vb-ib8i-sub9: vxmqwowq bce366e3 (empty) (no description set)
go-skill-vb-ogwh-sub8: wytvmorr 255a44af (empty) (no description set)
go-skill-vb-qi37-10: sqymymkp 575a3f0a (empty) (no description set)
go-skill-vb-qi37-11: ywvzwmnz bd04c730 (empty) (no description set)
go-skill-vb-qi37-6: krtroxux 6becf5ad (conflict) (no description set)
go-skill-vb-qi37-9: kyzmmklv 87c0ba1a go-skill-vb-qi37-9 | chore(vb-qi37.9): add landing report
go-skill-vb-tw3b: xnwkwvyr 994de93a (empty) (no description set)
holzman-workspace-1: ymuuwnyx cadd93fd (empty) (no description set)
holzman-workspace-10: yykqnmyr 6c628fe9 (empty) (no description set)
holzman-workspace-11: kzvxvorx d5e0efbf (empty) (no description set)
holzman-workspace-12: soyymsyu 93ae1ad4 (empty) (no description set)
holzman-workspace-2: xyttpuqy d6101242 (empty) (no description set)
holzman-workspace-3: zzqvnkyt d4f1dd4e (empty) (no description set)
holzman-workspace-4: krpwokoq be17223c (empty) (no description set)
holzman-workspace-5: xpttqyyq 0a7df103 (empty) (no description set)
holzman-workspace-6: xpzzoqxp 2d6b4b9e (empty) (no description set)
holzman-workspace-7: yywtnlzk 072d3f93 (empty) (no description set)
holzman-workspace-8: tmsxnpxl f6826da1 (empty) (no description set)
holzman-workspace-9: pkvsqpxk ea7797bb (empty) (no description set)
vb-0253-2: tvsmnuqs 2507ef04 (empty) (no description set)
vb-core-proof-15-gate: rtnolzxy e400a32c (empty) (no description set)
vb-core-proof-15-gate-recover: wtxxqwyw 1d343ee6 (empty) (no description set)
vb-iucs-recover2: vrnqttuu 2270665e (empty) (no description set)
vb-wg64: knvxlymp 6a67461c (empty) (no description set)
velvet-ballistics-holzman: pvzrzssp 61bae7ac (empty) (no description set)
velvet-ballistics-proof-repair: vnpuyvrs 361788d7 proof: bind verification to exec behavior
velvet-ballistics-split-workspace: vqnoqzwo 21645d19 (no description set)

```

### bd show vb-kyyf --json

exit=1 cwd=/home/lewis/src/femdation-vb-kyyf-bdd

```text
Warning: /home/lewis/src/femdation-vb-kyyf-bdd/.beads has permissions 0755 (recommended: 0700). Run: chmod 700 /home/lewis/src/femdation-vb-kyyf-bdd/.beads
Error fetching vb-kyyf: failed to search issues: search issues: search issues: Error 1146 (HY000): table not found: issues
{
  "error": "no issues found matching the provided IDs"
}

```

## Baseline classification note

Known repository may contain unrelated in-progress workspace/global debt; classify later State 11 failures via go-skill BLOCK_LOCAL/BLOCK_REGRESSION/BLOCK_RELEASE/REQUIRED_OBLIGATION_FAIL/DEFERRED_GLOBAL rules, not by hiding them here.
