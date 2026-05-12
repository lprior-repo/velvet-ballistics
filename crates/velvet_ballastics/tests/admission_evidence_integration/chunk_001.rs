#![forbid(unsafe_code)]
//! Admission and evidence chain integration tests.
//!
//! These tests exercise end-to-end flows across multiple crates: submitting
//! artifacts, running workflows under various policies, verifying journal
//! evidence chains, capability enforcement, budget validation, and taint
//! propagation.

use std::num::NonZeroUsize;
use std::sync::Arc;

use vb_core::ids::{ActionId, ConstIdx, ExprIdx, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{
    CompiledNode, CompiledNodeKind, CompiledWorkflow, ExprOp, ExprProgram, ResourceContract,
    WorkflowParts,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fail_assert(_message: std::fmt::Arguments<'_>) -> bool {
    false
}

macro_rules! fail_assert {
    ($($arg:tt)*) => {
        assert!(fail_assert(format_args!($($arg)*)), $($arg)*)
    }
}

/// Creates a simple two-node workflow: SetConst(42) -> Finish(result=slot0).
fn set_const_finish_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(0)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::SetConst {
            value: ConstIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(0),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("set_finish"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([ConstValue::I64(42)]),
        slot_count: 2,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: ResourceContract::DEFAULT,
        step_names: Box::default(),
    };
    CompiledWorkflow::try_from_parts(parts).ok()
}

/// Creates a workflow with a Do node requiring action 7.
fn do_action_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
    let node0 = CompiledNode {
        id: StepIdx::new(0),
        output: Some(SlotIdx::new(1)),
        next: Some(StepIdx::new(1)),
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Do {
            action: ActionId::new(7),
            input: SlotIdx::new(0),
        },
    };
    let node1 = CompiledNode {
        id: StepIdx::new(1),
        output: None,
        next: None,
        on_error: None,
        error_slot: None,
        kind: CompiledNodeKind::Finish {
            result: SlotIdx::new(1),
        },
    };
    let parts = WorkflowParts {
        name: Box::from("do_action"),
        digest,
        nodes: Box::from([node0, node1]),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count: 2,
        symbols_count: 0,
}
fn eval_expr_taint_workflow(digest: WorkflowDigest) -> Option<CompiledWorkflow> {
}
fn test_config() -> vb_runtime::shard::ShardConfig {
    }
}
fn temp_journal() -> Option<(tempfile::TempDir, Arc<vb_storage::FjallJournal>)> {
}
fn submit_artifact_then_run_succeeds() {
        }
        }
        }
    }
        }
        }
    }
        }
    }
        }
        }
    }
}
fn run_without_artifact_under_relaxed_policy() {
        }
        }
    }
        }
    }
        }
        }
    }
}
fn evidence_chain_after_execution() {
        }
    }
        }
        }
    }
        }
    }
        }
        }
    }
        }
            {
            }
            }
            {
            }
            }
        }
    }
}
fn capability_check_rejects_unauthorized_action() {
        }
    }
        }
        }
    }
        }
        }
    }
}
fn budget_validation_rejects_oversized_workflow() {
        }
        }
        }
    }
}
fn taint_propagates_through_expression_eval() {
        }
        }
    }
        }
        }
    }
        }
        }
    }
        }
        }
    }
        }
        }
        }
    }
}
[626 more lines]