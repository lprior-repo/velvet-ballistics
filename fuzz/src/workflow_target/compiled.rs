//! Compiled IR fuzz target body.

use super::node_slots::check_node_slots;

pub fn fuzz_compiled_ir(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let digest_before = parts.digest;
        let node_count_before = parts.nodes.len();
        let slot_count = parts.slot_count;
        let result = vb_core::CompiledWorkflow::try_from_parts(parts);
        if let Ok(workflow) = result {
            assert!(workflow.node_count() >= 1);
            assert_eq!(workflow.slot_count(), slot_count);
            assert_eq!(workflow.digest(), digest_before);
            assert_eq!(usize::from(workflow.node_count()), node_count_before);
            for i in 0..workflow.node_count() {
                let step = vb_core::StepIdx::new(i);
                let Some(node) = workflow.node(step) else {
                    continue;
                };
                if let Some(output) = node.output {
                    assert!(output.get() < slot_count);
                }
                check_node_slots(&node.kind, slot_count, i);
            }
        }
    }
}
