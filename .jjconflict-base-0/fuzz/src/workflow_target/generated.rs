//! Generated IR comparison fuzz target body.

pub fn fuzz_generated_compare(data: &[u8]) {
    if let Ok(parts) = postcard::from_bytes::<vb_core::WorkflowParts>(data) {
        let parts_clone = parts.clone();
        let validated = vb_core::validate_compiled_workflow(&parts);
        let workflow = vb_core::CompiledWorkflow::try_from_parts(parts);
        assert_eq!(validated.is_ok(), workflow.is_ok());
        if let (Ok(w1), Ok(w2)) = (
            workflow,
            vb_core::CompiledWorkflow::try_from_parts(parts_clone),
        ) {
            assert_eq!(w1.digest(), w2.digest());
            assert_eq!(w1.node_count(), w2.node_count());
            assert_eq!(w1.slot_count(), w2.slot_count());
        }
    }
}
