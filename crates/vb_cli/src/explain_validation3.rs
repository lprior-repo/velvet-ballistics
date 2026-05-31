//! Validation error formatting (part 3).
        ValidationError::ExpressionStackExceeded { declared, limit } => {
            outln!("Expression Stack Exceeded");
            outln!("  Expression stack depth {declared} exceeds limit {limit}.");
            explain_repair_hint(
                "validation",
                &[
                    "Simplify nested expressions",
                    "Break complex expressions into separate steps",
                ],
            );
        }
        ValidationError::ExpressionStackMismatch {
            expr_index,
            declared,
            computed,
        } => {
            outln!("Expression Stack Mismatch");
            outln!(
                "  Expression {expr_index}: declared {declared} stack slots, computed {computed}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the expression to declare the correct number of stack slots",
                    "Check expression syntax for stack manipulation operations",
                ],
            );
        }
        ValidationError::AccessorSlotOutOfRange {
            accessor_index,
            slot,
            slot_count,
        } => {
            outln!("Accessor Slot Out of Range");
            outln!(
                "  Accessor {accessor_index} references slot {slot}, but slot_count is {slot_count}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the slot reference to be within slot_count",
                    "Slot indices are zero-based",
                ],
            );
        }
        ValidationError::AccessorPathInvalid {
            accessor_index,
            segment_index,
        } => {
            outln!("Accessor Path Invalid");
            outln!("  Accessor {accessor_index} has invalid segment at index {segment_index}.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the accessor path syntax",
                    "Check the Velvet v1 schema for accessor path format",
                ],
            );
        }
        ValidationError::SlotReferenceOutOfRange {
            slot,
            slot_count,
            context,
        } => {
            outln!("Slot Reference Out of Range");
            outln!(
                "  Slot {slot} is out of range (slot_count={slot_count}) in context: {context}."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix the slot reference to be within the valid range",
                    "Ensure the slot exists in the workflow's slot schema",
                ],
            );
        }
        ValidationError::LoopBodyStepOutOfRange {
            step,
            node_count,
            source_node,
            label: _,
        } => {
            outln!("Loop Body Step Out of Range");
            outln!(
                "  Step {step}: loop body step out of range (node_count={node_count}, source_node={source_node})."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Fix loop body step references to be within node_count",
                    "Ensure loop body steps exist in the workflow",
                ],
            );
        }
        ValidationError::SlotDependencyCycle { slot, chain } => {
            outln!("Slot Dependency Cycle");
            outln!("  Slot {slot} has a dependency cycle: {chain}.");
            explain_repair_hint(
                "validation",
                &[
                    "Break the slot dependency cycle",
                    "Remove circular dependencies between slots",
                ],
            );
        }
        ValidationError::NodeKindConstraintViolation { node_index, detail } => {
            outln!("Node Kind Constraint Violation");
            outln!("  Node {node_index}: {detail}.");
            explain_repair_hint(
                "validation",
                &[
                    "Fix the node to comply with its kind constraints",
                    "Check the Velvet v1 schema for node kind rules",
                ],
            );
        }
        ValidationError::ActionContractMissing {
            action_id,
            node_index,
        } => {
            outln!("Action Contract Missing");
            outln!(
                "  Do node {node_index} references action_id {action_id}, which has no contract."
            );
            explain_repair_hint(
                "validation",
                &[
                    "Register an action contract for action_id {action_id}",
                    "All Do nodes must reference registered action contracts",
                ],
            );
        }
        ValidationError::ActionContractOrphan { action_id } => {
            outln!("Action Contract Orphan");
            outln!("  Action contract {action_id} has no corresponding Do node.");
            explain_repair_hint(
                "validation",
                &[
                    "Remove the orphan action contract",
                    "Or add a Do node that uses this action_id",
                ],
            );
        }
        ValidationError::SlotTypeInconsistency { slot } => {
            outln!("Slot Type Inconsistency");
            outln!("  Slot {slot} has writers with incompatible type kinds.");
            explain_repair_hint(
                "validation",
                &[
                    "Ensure all writers to this slot produce the same type",
                    "Fix type mismatches between step outputs",
                ],
            );
        }
        ValidationError::NonDeterministicPath { from_node, to_node } => {
            outln!("Non-Deterministic Path");
            outln!("  Path from node {from_node} to {to_node} contains no suspension point.");
            explain_repair_hint(
                "validation",
                &[
                    "Add a suspension point (ask, wait, or retry) to the path",
                    "Non-deterministic paths without suspension points cause replay issues",
                ],
            );
