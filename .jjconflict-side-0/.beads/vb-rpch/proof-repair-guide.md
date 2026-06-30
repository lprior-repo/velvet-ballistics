# Proof Repair Guide — vb-rpch (Attempt 4 → 5)

## Rejection Reason

The Kani harness in Attempt 4 was improved (field names corrected, `kani::any()` used for all present fields) but a new structural defect was introduced: **the harness only generates 11 of 18 JournalEvent variants** due to:
1. `kani::any::<u8>() % 11` only produces discriminants 0-10
2. Enum variant ordering mismatch between harness assumptions and actual `JournalEvent` enum

## Required Repairs

### 1. Fix JournalEvent kani::Arbitrary — Cover All 18 Variants (HIGH)

**Current code** (lines 33-107):
```rust
impl kani::Arbitrary for JournalEvent {
    fn any() -> Self {
        let discriminant: u8 = kani::any::<u8>() % 11;  // WRONG: only 0-10
        let run = RunId::new(kani::any());
        let seq = EventSeq::new(kani::any());
        match discriminant {
            0 => JournalEvent::RunAccepted { run, seq, workflow: ... },
            1 => JournalEvent::StepStarted { run, seq, step: ..., attempt: ... },
            // ... only covers 0-10
            _ => JournalEvent::RunCancelled { run, seq, attempt: ..., reason: None },
        }
    }
}
```

**Required code**:
```rust
impl kani::Arbitrary for JournalEvent {
    fn any() -> Self {
        let discriminant: u8 = kani::any::<u8>() % 18;  // FIXED: 0-17 for all 18 variants
        let run = RunId::new(kani::any());
        let seq = EventSeq::new(kani::any());
        match discriminant {
            0  => JournalEvent::RunAccepted { run, seq, workflow: WorkflowDigest::from_bytes(kani::any()) },
            1  => JournalEvent::RunAdmission { run, seq, artifact_digest: WorkflowDigest::from_bytes(kani::any()), granted_capabilities: kani::any(), policy: kani::any() },
            2  => JournalEvent::StepStarted { run, seq, step: StepIdx::new(kani::any()), attempt: kani::any() },
            3  => JournalEvent::StepSucceeded { run, seq, step: StepIdx::new(kani::any()), output: SlotIdx::new(kani::any()) },
            4  => JournalEvent::ActionScheduled { run, seq, action: ActionId::new(kani::any()), step: StepIdx::new(kani::any()), attempt: kani::any() },
            5  => JournalEvent::ActionCompletedEvent { run, seq, action: ActionId::new(kani::any()), step: StepIdx::new(kani::any()), attempt: kani::any() },
            6  => JournalEvent::ActionFailedEvent { run, seq, action: ActionId::new(kani::any()), step: StepIdx::new(kani::any()), attempt: kani::any() },
            7  => JournalEvent::SlotWrittenEvent { run, seq, slot: SlotIdx::new(kani::any()), value: Some(kani::any()), extra: Some(kani::any()), attempt: kani::any() },
            8  => JournalEvent::WaitScheduledEvent { run, seq, step: StepIdx::new(kani::any()), attempt: kani::any() },
            9  => JournalEvent::AskScheduledEvent { run, seq, step: StepIdx::new(kani::any()), attempt: kani::any() },
            10 => JournalEvent::AskAnsweredEvent { run, seq, step: StepIdx::new(kani::any()), attempt: kani::any() },
            11 => JournalEvent::RetryScheduledEvent { run, seq, step: StepIdx::new(kani::any()), attempt: kani::any() },
            12 => JournalEvent::RunCancelled { run, seq, attempt: kani::any(), reason: Some(kani::any()) },
            13 => JournalEvent::RunFinished { run, seq, result: SlotIdx::new(kani::any()), attempt: kani::any() },
            14 => JournalEvent::RunFailedEvent { run, seq, attempt: kani::any() },
            15 => JournalEvent::RunResumed { run, seq, timestamp: kani::any() },
            16 => JournalEvent::RunRetried { run, seq, timestamp: kani::any() },
            17 => JournalEvent::RunAnswered { run, seq, slot_idx: SlotIdx::new(kani::any()), answer: kani::any(), timestamp: kani::any() },
        }
    }
}
```

### 2. Verify Against events.rs Enum Definition

The enum definition is in `/home/lewis/src/velvet-ballistics/crates/vb_storage/src/events.rs`:
- Lines 15-22: `RunAccepted { run, seq, workflow }`
- Lines 24-35: `RunAdmission { run, seq, artifact_digest, granted_capabilities, policy }`
- Lines 37-46: `StepStarted { run, seq, step, attempt }`
- Lines 48-57: `StepSucceeded { run, seq, step, output }`
- Lines 59-70: `ActionScheduled { run, seq, action, step, attempt }`
- Lines 72-83: `ActionCompletedEvent { run, seq, action, step, attempt }`
- Lines 85-96: `ActionFailedEvent { run, seq, action, step, attempt }`
- Lines 98-112: `SlotWrittenEvent { run, seq, slot, value, extra, attempt }`
- Lines 114-123: `WaitScheduledEvent { run, seq, step, attempt }`
- Lines 125-134: `AskScheduledEvent { run, seq, step, attempt }`
- Lines 136-145: `AskAnsweredEvent { run, seq, step, attempt }`
- Lines 147-156: `RetryScheduledEvent { run, seq, step, attempt }`
- Lines 158-167: `RunCancelled { run, seq, attempt, reason }`
- Lines 169-178: `RunFinished { run, seq, result, attempt }`
- Lines 180-187: `RunFailedEvent { run, seq, attempt }`
- Lines 189-201: `RunResumed { run, timestamp }`
- Lines 203-212: `RunAnswered { run, slot_idx, answer, timestamp }`

### 3. Run cargo kani Verification

After fixing the harness, run:
```bash
cd /home/lewis/src/velvet-ballistics && cargo kani --package vb_storage -- --harness hydrate_run_frame_precond_kani 2>&1 | head -50
```

Show output in proof-evidence.md proving:
- Harness compiles
- No panic in explored states
- Coverage report

### 4. Update proof-evidence.md

Add Kani execution output section:
```markdown
## Kani Execution

```
$ cargo kani --package vb_storage -- --harness hydrate_run_frame_precond_kani
[kani output]
```
```

## Unchanged from Attempt 4

These remain appropriately documented and do not need changes:
- **Verus BLOCKED_TOOLING**: Nightly not installed — waiver applies
- **TLC WAIVER for PO-VB-008 through PO-VB-013**: Simulation mode acceptable given state space explosion
- **GAP-3 Waivers**: Sound and appropriately deferred

## Checklist for Attempt 5

- [ ] JournalEvent `kani::any()` uses `% 18` not `% 11`
- [ ] All 18 enum variants are covered in match arms
- [ ] Variant ordering matches events.rs definition
- [ ] `cargo kani` runs and produces output
- [ ] proof-evidence.md updated with Kani execution evidence
- [ ] proof-findings.jsonl updated with SEV-1-KANI-001/002/003 status changes
