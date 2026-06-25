#![no_main]

use libfuzzer_sys::fuzz_target;
use xtask::doc_reconcile::reconcile::check_doc_taint_consistency;
use xtask::doc_reconcile::{DocReconcileError, ResolvedNode};

fuzz_target!(|data: &[u8]| {
    let arbitrary = String::from_utf8_lossy(data);

    drop(check_doc_taint_consistency(&arbitrary));

    assert_eq!(
        check_doc_taint_consistency(
            "| EvalExpr | Always Clean — no taint join of expression operands. |"
        ),
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "Always Clean".to_owned(),
        })
    );

    assert_eq!(
        check_doc_taint_consistency("| BuildObject | Always Clean — no join of field taints |"),
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildObject,
            phrase: "Always Clean".to_owned(),
        })
    );

    assert_eq!(
        check_doc_taint_consistency("| BuildList | Always Clean — no join of item taints |"),
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::BuildList,
            phrase: "Always Clean".to_owned(),
        })
    );

    assert_eq!(
        check_doc_taint_consistency("EvalExpr writes with write_slot (not write_slot_with_taint)"),
        Err(DocReconcileError::StaleCleanOnlyTaintText {
            node: ResolvedNode::EvalExpr,
            phrase: "write_slot".to_owned(),
        })
    );
});
