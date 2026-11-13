//! Budget module integration tests.

use crate::budget::{
    AggregateBudgetError, AggregateResourceBudget, AggregateResourceCapacity,
    AggregateResourceUsage, BoundednessPolicy, BudgetError, BudgetExhaustionOutcome,
    LifetimeBudgetError, RemainingBudget, RemainingBudgetError, RunLifetimeBudget,
    ShardUsageSnapshot, WholeWorkflowBudget, check_global_quota, check_tenant_quota,
    release_global_quota, validate_aggregate_budget,
};
use crate::engine::StepBudget;
use crate::ids::{ActionId, ConstIdx, ExprIdx, SlotIdx, StepIdx, TenantId};
use crate::workflow::{
    CompiledNode, CompiledNodeKind, ExprBranch, NodeContinuation, ResourceContract, SlotBranch,
    WorkflowError, node,
};

include!("tests/basic.rs");

include!("tests/loop_regression.rs");

include!("tests/blackhat.rs");

include!("tests/comprehensive_a.rs");

include!("tests/comprehensive_b.rs");

include!("tests/edge_cases.rs");

include!("tests/overflow.rs");

include!("tests/count_push_a.rs");

include!("tests/count_push_b.rs");

include!("tests/count_push_c1.rs");

include!("tests/count_push_c2.rs");

include!("tests/mutation_killing.rs");

include!("tests/budget_dims.rs");

include!("tests/lifetime.rs");

include!("tests/whole_workflow.rs");
