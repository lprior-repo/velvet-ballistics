fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = b"version: velvet-ballistics/v1\nname: legacy_wire_fixture\nwhen: { manual: {} }\nsteps:\n  - id: make\n    set: { output: answer, value: \"42\" }\n  - id: done\n    finish: { result: answer }\n";
    let workflow = vb_compile::compile_workflow(source)?;
    let artifact = vb_storage::admission::prepare_compiled_artifact(&workflow, vb_core::RuntimePolicy::Strict, &[])?;
    let root = std::path::Path::new(".beads/vb-9agmk/schema2-fixture");
    std::fs::write(root.join("source.yaml"), source)?;
    std::fs::write(root.join("artifact.postcard"), postcard::to_allocvec(&artifact)?)?;
    std::fs::write(root.join("workflow-parts.postcard"), &artifact.ir)?;
    println!("artifact={:?} workflow={:?} ir_bytes={}", artifact.digest, workflow.digest(), artifact.ir.len());
    Ok(())
}
