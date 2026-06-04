#![forbid(unsafe_code)]
//! Small digest comparison helpers for recovery checks.

use vb_core::{ActionId, StepIdx, WorkflowDigest};

#[must_use]
pub(crate) fn workflow_digest_bytes_equal(left: WorkflowDigest, right: WorkflowDigest) -> bool {
    let [
        l0,
        l1,
        l2,
        l3,
        l4,
        l5,
        l6,
        l7,
        l8,
        l9,
        l10,
        l11,
        l12,
        l13,
        l14,
        l15,
        l16,
        l17,
        l18,
        l19,
        l20,
        l21,
        l22,
        l23,
        l24,
        l25,
        l26,
        l27,
        l28,
        l29,
        l30,
        l31,
    ] = left.as_bytes();
    let [
        r0,
        r1,
        r2,
        r3,
        r4,
        r5,
        r6,
        r7,
        r8,
        r9,
        r10,
        r11,
        r12,
        r13,
        r14,
        r15,
        r16,
        r17,
        r18,
        r19,
        r20,
        r21,
        r22,
        r23,
        r24,
        r25,
        r26,
        r27,
        r28,
        r29,
        r30,
        r31,
    ] = right.as_bytes();

    l0 == r0
        && l1 == r1
        && l2 == r2
        && l3 == r3
        && l4 == r4
        && l5 == r5
        && l6 == r6
        && l7 == r7
        && l8 == r8
        && l9 == r9
        && l10 == r10
        && l11 == r11
        && l12 == r12
        && l13 == r13
        && l14 == r14
        && l15 == r15
        && l16 == r16
        && l17 == r17
        && l18 == r18
        && l19 == r19
        && l20 == r20
        && l21 == r21
        && l22 == r22
        && l23 == r23
        && l24 == r24
        && l25 == r25
        && l26 == r26
        && l27 == r27
        && l28 == r28
        && l29 == r29
        && l30 == r30
        && l31 == r31
}

pub(crate) fn first_action_abi_mismatch(
    entries: &[(ActionId, WorkflowDigest, WorkflowDigest)],
) -> Option<(ActionId, WorkflowDigest, WorkflowDigest)> {
    entries.iter().find_map(|(action_id, expected, found)| {
        if workflow_digest_bytes_equal(*expected, *found) {
            None
        } else {
            Some((*action_id, *expected, *found))
        }
    })
}

pub(crate) fn first_policy_mismatch(
    entries: &[(StepIdx, WorkflowDigest, WorkflowDigest)],
) -> Option<(StepIdx, WorkflowDigest, WorkflowDigest)> {
    entries.iter().find_map(|(step, expected, found)| {
        if workflow_digest_bytes_equal(*expected, *found) {
            None
        } else {
            Some((*step, *expected, *found))
        }
    })
}
