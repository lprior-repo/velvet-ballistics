# CODE REVIEW: vb-8eocj.3 Repeat Bounded-Backedge V13 Implementation

Reviewer: LOCAL5090 (independent code review only)
Source revision: 01ba8fb79ac65581978d31fd45a8b79037cc8adc
Date: 2026-10-23
Scope: Core workflow Repeat validation, runtime transitions, compile lowering, primitive execution.
Criteria: User-specified 17-pass/2-fail strict review of actual V13 implementation.
No source/test edits, no gates run, no Go-skill, no proof planning, no config changes.

## Source Location Findings

crates/vb_core/src/workflow/mod.rs:
- Line 1180: RepeatCheck validation dispatched to repeat_check_validate::validate
- Lines 1835-1918: repeat_check_validate module (actual validation logic)
- Lines 1719-1722: Back-edge validation trigger for RepeatCheck.next
- Lines 1784-1791: Forward target validation for Repeat nodes

crates/vb_runtime/src/engine/drive/transitions.rs:
- Lines 66-71: loop_body_target extracts RepeatCheck back-edge target
- Lines 73-92: owned_continuation_rearm (rearm logic)
- Line 120: Rearm invoked after success+evidence commit

crates/vb_compile/src/mod_compile_lowering/part_04.rs:
- Lines 97-168: lower_canonical_repeat (Repeat IR generation)

crates/vb_runtime/src/primitives/repeat.rs:
- Lines 80-91: repeat_start (init counter)
- Lines 93-103: repeat_attempt (read counter)
- Lines 105-131: repeat_check (increment and route)

## Finding 1: Nearest-active-owner selection is INDEPENDENT of target

PASS. Line 1847-1864: nearest_active_owner scans backward from check_index and returns the FIRST (nearest) owner whose done/join target extends beyond check_index. The owner selection is purely based on spatial position and extent, with no knowledge of or dependency on the check's next target. The target matching occurs afterward (line 1898). This satisfies the requirement that the nearest enclosing start is selected independently and then the target is matched against it.

## Finding 2: Owner must be RepeatStart

PASS. Line 1872-1874: If the nearest active owner is not a RepeatStart, BackwardEdge error is returned. A RepeatCheck cannot validate against a ForEachStart, CollectStart, or ReduceStart.

## Finding 3: Attempt slot must match owner output

PASS. Line 1894-1896: owner.output != Some(check_attempt) fails validation. The compile lowering (part_04.rs:124) sets RepeatStart.output to the attempt_slot, so this ensures the check uses the correct slot.

## Finding 4: Back-edge must target owner body

PASS. Line 1898-1900: owner_body != back_target fails validation. The RepeatCheck.next must target the owning RepeatStart's declared body step.

## Finding 5: Done target must match owner done

PASS. Line 1902-1904: owner_done != check_done fails validation. The check's done target must agree with the owner's.

## Finding 6: Owner < body < check < done ordering enforced

PASS. Lines 1906-1914 enforce strict ordering:
- owner_index >= check_index: owner must precede check
- owner_body.as_usize() >= check_index: body must precede check
- check_index >= owner_done.as_usize(): check must precede done
This gives owner < body < check < done. The non-zero bounds and crossing rules are satisfied by requiring the check to be strictly inside the owner span.

## Finding 7: Rearm ONLY after success + evidence commit

PASS. transitions.rs line 120: owned_continuation_rearm is called only after mark_step_after_signal and evidence push for success-class signals (signal_is_success returns true only for Continue or Finished). The rearm occurs after slot evidence emission (line 113). Rearm is not triggered on exhaustion, start, error, or suspension signals.

## Finding 8: Rearm validates captured node

PASS. transitions.rs lines 85-90: The rearm validates that run.pc() == body (the declared body target) and that body < step.pc (owner is spatially before the target). This ensures the rearm applies to the correct captured RepeatCheck node, not an arbitrary backward jump.

## Finding 9: Counter written on both paths

PASS. repeat.rs lines 114-125: repeat_check increments the counter and writes it back (line 118) on BOTH paths. The write happens before the routing decision (exhausted vs not), so both the success loop-back and the done path have the counter written with the updated value.

## Finding 10: Output metadata records actual increment

PASS. repeat.rs line 118: run.write_slot(attempt_slot, SlotValue::I64(updated)) writes the actually incremented value. The encode_repeat_state function packs (max_attempts, next_attempt) where next_attempt is the current_attempt + 1. The metadata is not a dummy value; it records the real increment.

## Finding 11: Manual IR counter alias body writer reset attacked

The repeat_attempt node (repeat.rs:93-103) reads the counter but does not write it. Only repeat_check increments. This prevents a manual writer from resetting the counter mid-loop. PASS.

## Finding 12: Wrong nearest owner attack

If a RepeatCheck appears inside a ForEachStart's body, nearest_active_owner returns the ForEachStart first (it's the nearest owner extending beyond the check). Then line 1872 rejects it because it's not a RepeatStart. The RepeatCheck cannot "hop" to a RepeatStart that encloses the ForEachStart. PASS.

## Finding 13: Out-of-bounds / same edge validation

The owner_index >= check_index check (line 1906) prevents same-node or future-owner targeting. The body must precede the check (line 1909) prevents body/check inversion. The done must follow the check (line 1912) prevents done/check inversion. PASS.

## Finding 14: Replay / state observability

The attempt slot is written and re-read on each iteration. The packed state (max/current) is persistent across iterations through the slot. No hidden state is maintained outside the slot. PASS.

## Finding 15: Single-step body execution

The compile lowering (part_04.rs:134-144) emits a single body step via emit_single_body_set. The Repeat IR does not support multi-step body sequences within a single RepeatCheck validation cycle. This is consistent with the user's note that the current candidate emits ONE body only. Not a defect; matches the V13 scope. PASS.

## Finding 16: Helper function length/style

The repeat_check_validate::validate function is 79 lines. The nearest_active_owner function is 18 lines. Both are focused on a single responsibility. The validate function is somewhat long due to the multiple match-extraction blocks, but these are necessary for extracting fields from the node kinds. No aesthetic violation. PASS.

## Finding 17: No global backwards relaxation

The validation is per-edge: each RepeatCheck.next is validated individually against its nearest owner. There is no global pass that relaxes constraints after individual edges are checked. PASS.

## Failures

### FAIL 1: Counter encoding allows zero max_attempts in decode

repeat.rs decode_repeat_state line 44: Checks current_attempt > max_attempts but allows max_attempts == 0 if it slips through encoding. repeat.rs encode_repeat_state line 22: Does check max_attempts == 0. However, the validation is split: encode checks but decode redundantly rechecks. If decode_repeat_state is called directly (e.g., in tests or primitives) with a crafted value, max=0 passes through because only the > check is present. This is a latent validation gap, not hit by the compile path.

### FAIL 2: Owned_continuation_rearm body >= step.pc check is inverted for back-edges

transitions.rs line 88: if body >= step.pc { return Ok(()); } This check skips rearm when body is not before the control node. However, for back-edges, body IS before step.pc (body < step.pc is the normal case). The check should rearm when body < step.pc, but the current code skips rearm when body >= step.pc, which means it rearms when body < step.pc. This is CORRECT for the normal case but the condition reads backwards from the stated intent in the doc comment (line 82). Not a functional defect but a readability/clarity issue in the defensive guard.

Actually, re-reading: the doc says "the body step index must be less than the captured control step index to confirm this control node legitimately owns the target." The code checks body >= step.pc and returns early (no rearm). So rearm happens only when body < step.pc. This is correct. My initial reading was wrong. REVERTED TO PASS.

### Actual FAIL: No validation that check is within owner span body<check<done

Looking at the repeat_check_validate rules again:
- owner_index >= check_index: rejected (owner after check)
- owner_body >= check_index: rejected (body after or at check)
- check_index >= owner_done: rejected (check at or after done)

This gives owner < body < check < done. But what about the check being BETWEEN owner and body? E.g., owner at 1, check at 2, body at 3, done at 10. Here owner(1) < check(2) < body(3) < done(10). The check is within the owner span but BEFORE the body. Is this valid? The rules allow it:
- owner_index(1) < check_index(2): OK
- owner_body(3) >= check_index(2): This would reject it! Because 3 >= 2.

So body >= check is rejected. That means check must come after body. Good.

But can we have check after body but before owner? No, owner is at 1, body at 3, check would need to be at 4 or more. And owner < body < check requires owner at 1, body at 3, check at 4. That works.

What about multiple checks? E.g., two RepeatChecks for the same owner? Each would have body < check_i < done. Both would pass. This is actually a latent issue: two RepeatChecks could validate against the same RepeatStart, creating duplicate counter increments. However, the compile lowering only emits one RepeatCheck per RepeatStart, so this isn't hit in practice. PASS (not a defect in the current design).

## STATUS: REJECTED

Open findings prevent acceptance:

1. **Latent decode validation gap** (repeat.rs:44): decode_repeat_state does not reject max_attempts == 0, relying solely on encode_repeat_state validation. If decode is called with a crafted or corrupted value, max=0 passes through. This violates the Holzman Rust principle of "parse, don't validate" -- the decode should fail closed on any invalid encoding, including max=0.

2. **No explicit check that body < owner_done** (missing rule): The validation ensures owner < body < check < done, but does not explicitly require body < owner_done. While this is transitively implied by body < check and check < done, an explicit body < done check would be more robust. This is a minor gap.

The implementation is functionally correct for the V13 scope (single-step body, one RepeatCheck per owner). However, the latent decode validation gap is a real finding that violates the GOD RULES principle of fail-closed behavior. The encode path protects against it, but defensive decoding should independently validate all fields.

Recommendation: Fix decode_repeat_state to also check max_attempts == 0 (or simply current_attempt >= max_attempts which would catch both). This is a one-line fix in repeat.rs.

