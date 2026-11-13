//! vb-eirmk.4.8: public compiler entry and artifact emission boundary.
//!
//! Qualifies `compile_workflow`, `YamlCompiler::compile`, `parse_ast` parity,
//! deterministic digest, and complete artifact emission for every scored
//! family through public APIs only. Master `function:28:1`, `function:28:6`
//! and §51 digest fields are traced to failing-before/passing-after behavior
//! without adding lowering internals.
//!
//! Contract: `.beads/vb-eirmk.4.8/contract.md` (pin `28d87a9f`).
#![forbid(unsafe_code)]

use vb_compile::{
    CompileError, YamlCompiler, canonical_digest, compile_source, compile_verified_artifact,
    compile_workflow, compute_compiled_digest, emit_compiled_artifact, parse_workflow_source,
};
use vb_core::{CompiledWorkflow, ResourceContract, WorkflowDigest, validate_compiled_workflow};

const HEADER: &str = "version: velvet-ballistics/v1\nname: entry_qual\nwhen: {manual: {}}\n";
const INPUTS: &str =
    "inputs:\n  a_items: {is: list, of: any, max: 4}\n  b_items: {is: list, of: any, max: 4}\n";
const SEED: &str = "steps:\n  - id: seed\n    set: {output: base, value: '42'}\n  - id: delay\n    set: {output: delay, value: '5'}\n";
const DONE: &str = "  - id: done\n    finish: {result: 7}\n";

fn compile(text: &str) -> Result<CompiledWorkflow, String> {
    compile_workflow(text.as_bytes()).map_err(|e| format!("compile: {e:?}"))
}

fn compile_err(text: &str) -> Result<vb_compile::CompileErrors, String> {
    match compile_workflow(text.as_bytes()) {
        Ok(_) => Err("unexpected compile acceptance".to_owned()),
        Err(errors) => Ok(errors),
    }
}

fn leaf() -> &'static str {
    "        - id: leaf\n          set: {output: picked, value: '1'}\n"
}

fn family_source(family: &str) -> String {
    match family {
        "set" => {
            format!("{HEADER}{SEED}  - id: s\n    set: {{output: picked, value: '1'}}\n{DONE}")
        }
        "save" => {
            format!("{HEADER}{SEED}  - id: s\n    save: {{output: picked, value: '1'}}\n{DONE}")
        }
        "do" => format!("{HEADER}{SEED}  - id: d\n    do: {{action: '0', input: '$base'}}\n{DONE}"),
        "run" => {
            format!("{HEADER}{SEED}  - id: d\n    run: {{action: '0', input: '$base'}}\n{DONE}")
        }
        "choose" => format!(
            "{HEADER}{SEED}  - id: c\n    choose:\n      branches:\n        - when: 'true'\n          steps:\n            - id: leaf\n              set: {{output: picked, value: '1'}}\n      otherwise: after_outer\n  - id: after_outer\n    set: {{output: after_outer_v, value: '3'}}\n{DONE}"
        ),
        "foreach" => format!(
            "{HEADER}{INPUTS}{SEED}  - id: loop\n    for_each:\n      input: '$input.a_items'\n      variable: entry\n      steps:\n{leaf}{DONE}",
            leaf = leaf()
        ),
        "together" => format!(
            "{HEADER}{SEED}  - id: t\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: leaf\n              set: {{output: picked, value: '1'}}\n{DONE}"
        ),
        "collect" => format!(
            "{HEADER}{INPUTS}{SEED}  - id: col\n    collect:\n      variable: obatch\n      source: '$input.a_items'\n      pages: 1\n      items: 1\n      steps:\n{leaf}{DONE}",
            leaf = leaf()
        ),
        "reduce" => format!(
            "{HEADER}{INPUTS}{SEED}  - id: fold\n    reduce:\n      variable: ototal\n      input: '$input.a_items'\n      initial: '0'\n      steps:\n        - id: leaf\n          set: {{output: picked, value: '1'}}\n        - id: update\n          set: {{output: local_value, value: '$ototal'}}\n{DONE}"
        ),
        "repeat" => format!(
            "{HEADER}{SEED}  - id: r\n    repeat:\n      max_attempts: 2\n      steps:\n{leaf}{DONE}",
            leaf = leaf()
        ),
        "wait" => format!("{HEADER}{SEED}  - id: w\n    wait: {{timeout: '$delay'}}\n{DONE}"),
        "ask" => format!("{HEADER}{SEED}  - id: q\n    ask: {{prompt: '$base'}}\n{DONE}"),
        "finish" => format!("{HEADER}steps:\n  - id: done\n    finish: {{result: 7}}\n"),
        _ => unreachable!(),
    }
}

const FAMILIES: [&str; 13] = [
    "set", "save", "do", "run", "choose", "foreach", "together", "collect", "reduce", "repeat",
    "wait", "ask", "finish",
];

fn assert_terminal(workflow: &CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    validate_compiled_workflow(&parts).map_err(|e| format!("validate: {e:?}"))?;
    let last = parts.nodes.last().ok_or("no nodes")?;
    match &last.kind {
        vb_core::CompiledNodeKind::Finish { .. } => Ok(()),
        other => Err(format!("last not Finish: {other:?}")),
    }
}

fn assert_emit_roundtrip(workflow: &CompiledWorkflow) -> Result<(), String> {
    let bytes = emit_compiled_artifact(workflow).map_err(|e| format!("emit: {e:?}"))?;
    let parts: vb_core::WorkflowParts =
        postcard::from_bytes(&bytes).map_err(|e| format!("decode: {e:?}"))?;
    let rebuilt =
        CompiledWorkflow::try_from_parts(parts.clone()).map_err(|e| format!("rebuild: {e:?}"))?;
    if rebuilt.to_parts() != parts {
        return Err("roundtrip parts differ".to_owned());
    }
    let reencoded =
        postcard::to_allocvec(&rebuilt.to_parts()).map_err(|e| format!("reencode: {e:?}"))?;
    if reencoded != bytes.as_ref() {
        return Err("reencoded bytes differ".to_owned());
    }
    Ok(())
}

#[test]
fn compile_workflow_and_yaml_compiler_compile_agree() -> Result<(), String> {
    for family in FAMILIES {
        let source = family_source(family);
        let direct = compile(&source)?;
        let via_compiler = YamlCompiler::default()
            .compile(source.as_bytes())
            .map_err(|e| format!("{family} compiler: {e:?}"))?;
        if direct.digest() != via_compiler.digest() {
            return Err(format!("{family}: digest mismatch"));
        }
        if direct.semantic_source_identity() != via_compiler.semantic_source_identity() {
            return Err(format!("{family}: semantic mismatch"));
        }
        if direct.to_parts() != via_compiler.to_parts() {
            return Err(format!("{family}: parts mismatch"));
        }
    }
    Ok(())
}

#[test]
fn entry_rejects_malformed_bytes_from_both_entry_points() -> Result<(), String> {
    let compiler = YamlCompiler::default();
    for source in [
        b"".as_slice(),
        b"{{{broken".as_slice(),
        b"\xff\xfe".as_slice(),
    ] {
        if compile_workflow(source).is_ok() {
            return Err("compile_workflow accepted malformed bytes".to_owned());
        }
        if compiler.compile(source).is_ok() {
            return Err("YamlCompiler::compile accepted malformed bytes".to_owned());
        }
        if compiler.parse_ast(source).is_ok() {
            return Err("parse_ast accepted malformed bytes".to_owned());
        }
    }
    Ok(())
}

#[test]
fn entry_canonical_validation_rejects_noncanonical() -> Result<(), String> {
    let cases = [
        "version: velvet-ballistics/v2\nname: bad\nwhen: {manual: {}}\nsteps:\n  - id: done\n    finish: {result: 7}\n",
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nsteps: []\n",
        "version: velvet-ballistics/v1\nname: Bad-Name\nwhen: {manual: {}}\nsteps:\n  - id: done\n    finish: {result: 7}\n",
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}, webhook: {}}\nsteps:\n  - id: done\n    finish: {result: 7}\n",
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nsteps:\n  - id: done\n    finish: {result: 7}\n  - id: done\n    finish: {result: 8}\n",
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nunknown: 1\nsteps:\n  - id: done\n    finish: {result: 7}\n",
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nsteps:\n  - id: s\n    set: {output: a, value: '1', bogus: 1}\n  - id: done\n    finish: {result: 7}\n",
    ];
    for source in cases {
        let escaped = format!("---\n{source}");
        if compile_workflow(escaped.as_bytes()).is_ok() {
            return Err(format!("compile accepted noncanonical: {source}"));
        }
        if YamlCompiler::default().compile(escaped.as_bytes()).is_ok() {
            return Err(format!("compiler accepted noncanonical: {source}"));
        }
    }
    let errs = compile_err(
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nsteps:\n  - id: s\n    nosuch: {}\n  - id: done\n    finish: {result: 7}\n",
    )?;
    let _ = errs;
    Ok(())
}

#[test]
fn entry_preserves_nested_together_rejection() -> Result<(), String> {
    let source = format!(
        "{HEADER}{SEED}  - id: t\n    together:\n      branches:\n        - label: a\n          steps:\n            - id: inner\n              together:\n                branches:\n                  - label: b\n                    steps:\n                      - id: leaf\n                        set: {{output: picked, value: '1'}}\n{DONE}"
    );
    let errors = match compile_workflow(source.as_bytes()) {
        Ok(_) => return Err("nested together accepted".to_owned()),
        Err(errors) => errors,
    };
    if errors.0.iter().any(|e| {
        matches!(
            e,
            CompileError::Workflow(vb_core::WorkflowError::NestedTogether { .. })
        )
    }) {
        Ok(())
    } else {
        Err(format!("wrong together diagnostic: {errors:?}"))
    }
}

#[test]
fn parse_ast_parity_accepts_valid_canonical_sources() -> Result<(), String> {
    // `parse_ast` validates the legacy step-shape dialect while `compile`
    // validates the canonical dialect (contract INV-07). Shared-dialect
    // families must agree on the same bytes; canonical-only families must
    // be accepted by `compile` and typed-rejected by the legacy gate so
    // the boundary never goes silent.
    for family in FAMILIES {
        let source = family_source(family);
        compile(&source)?;
        let shared = matches!(family, "set" | "save" | "repeat" | "finish");
        let parsed = YamlCompiler::default().parse_ast(source.as_bytes());
        if shared {
            parsed.map_err(|e| format!("{family}: {e:?}"))?;
        } else {
            let err = match parsed {
                Ok(_) => {
                    return Err(format!(
                        "{family}: legacy gate accepted canonical-only shape"
                    ));
                }
                Err(errors) => errors,
            };
            if err.0.is_empty() {
                return Err(format!("{family}: empty diagnostic"));
            }
        }
    }
    Ok(())
}

#[test]
fn parse_ast_parity_rejects_invalid_semantics() -> Result<(), String> {
    let cases = [
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nsteps:\n  - id: s\n    set: {output: a, value: '$nonexistent'}\n  - id: done\n    finish: {result: 7}\n",
        "version: velvet-ballistics/v1\nname: bad\nwhen: {manual: {}}\nsteps:\n  - id: a\n    set: {output: x, value: '1'}\n  - id: b\n    set: {output: x2, value: '$a'}\n",
    ];
    for source in cases {
        if YamlCompiler::default().parse_ast(source.as_bytes()).is_ok() {
            return Err(format!("parse_ast accepted invalid: {source}"));
        }
        if compile_workflow(source.as_bytes()).is_ok() {
            return Err(format!("compile accepted invalid: {source}"));
        }
    }
    Ok(())
}

#[test]
fn digest_fields_master_51() -> Result<(), String> {
    for family in FAMILIES {
        let source = family_source(family);
        let workflow = compile(&source)?;
        let bytes = emit_compiled_artifact(&workflow).map_err(|e| format!("emit: {e:?}"))?;
        // workflow_source_digest = BLAKE3(raw source bytes)
        let expected_source = WorkflowDigest::from_bytes(blake3::hash(source.as_bytes()).into());
        if compute_compiled_digest(source.as_bytes()) != expected_source {
            return Err(format!("{family}: source digest"));
        }
        // compiled_digest = BLAKE3(postcard parts with digest zeroed)
        let mut zeroed = workflow.to_parts();
        let claimed = zeroed.digest;
        zeroed.digest = WorkflowDigest::from_bytes([0u8; 32]);
        let zeroed_bytes = postcard::to_allocvec(&zeroed).map_err(|e| format!("encode: {e:?}"))?;
        let recomputed = WorkflowDigest::from_bytes(blake3::hash(&zeroed_bytes).into());
        if recomputed != claimed || recomputed != workflow.digest() {
            return Err(format!("{family}: compiled digest"));
        }
        // semantic identity = canonical_digest(source)
        let parsed = parse_workflow_source(&source).map_err(|e| format!("parse: {e:?}"))?;
        let canonical = canonical_digest(&parsed).map_err(|e| format!("canonical: {e:?}"))?;
        if workflow.semantic_source_identity() != Some(canonical) {
            return Err(format!("{family}: semantic identity"));
        }
        // payload_digest = BLAKE3(emitted postcard payload bytes)
        let from_source = compile_source(&parsed).map_err(|e| format!("direct: {e:?}"))?;
        let direct_bytes =
            emit_compiled_artifact(&from_source).map_err(|e| format!("emit2: {e:?}"))?;
        if direct_bytes != bytes {
            return Err(format!("{family}: direct emission mismatch"));
        }
    }
    Ok(())
}

#[test]
fn verified_artifact_digests_match_master_51() -> Result<(), String> {
    let source = family_source("set");
    let verified = compile_verified_artifact(source.as_bytes(), &[])
        .map_err(|e| format!("verified: {e:?}"))?;
    let payload = verified.payload();
    let payload_digest = WorkflowDigest::from_bytes(blake3::hash(payload).into());
    if payload_digest != verified.payload_digest() {
        return Err("payload_digest mismatch".to_owned());
    }
    let contracts: Vec<vb_core::ActionContract> = Vec::new();
    let contract_bytes =
        postcard::to_allocvec(&contracts).map_err(|e| format!("contracts: {e:?}"))?;
    let contract_digest = WorkflowDigest::from_bytes(blake3::hash(&contract_bytes).into());
    if contract_digest != verified.action_contract_digest() {
        return Err("action_contract_digest mismatch".to_owned());
    }
    let policy_bytes =
        postcard::to_allocvec(&ResourceContract::DEFAULT).map_err(|e| format!("policy: {e:?}"))?;
    let policy_digest = WorkflowDigest::from_bytes(blake3::hash(&policy_bytes).into());
    if policy_digest != verified.policy_digest() {
        return Err("policy_digest mismatch".to_owned());
    }
    let provenance = match verified.source_provenance() {
        vb_compile::SourceProvenance::CompiledSourceSpec(digest) => digest,
        other => return Err(format!("provenance: {other:?}")),
    };
    if provenance != compute_compiled_digest(source.as_bytes()) {
        return Err("provenance is not workflow_source_digest".to_owned());
    }
    Ok(())
}

#[test]
fn deterministic_digest() -> Result<(), String> {
    let source = family_source("foreach");
    let first = compile(&source)?;
    let first_emit = emit_compiled_artifact(&first).map_err(|e| format!("emit: {e:?}"))?;
    for _ in 0..3 {
        let again = compile(&source)?;
        if again.digest() != first.digest() {
            return Err("compiled digest not deterministic".to_owned());
        }
        if again.semantic_source_identity() != first.semantic_source_identity() {
            return Err("semantic identity not deterministic".to_owned());
        }
        let parsed = parse_workflow_source(&source).map_err(|e| format!("parse: {e:?}"))?;
        let canonical = canonical_digest(&parsed).map_err(|e| format!("canonical: {e:?}"))?;
        if Some(canonical) != first.semantic_source_identity() {
            return Err("canonical not deterministic".to_owned());
        }
        let again_emit = emit_compiled_artifact(&again).map_err(|e| format!("emit: {e:?}"))?;
        if again_emit != first_emit {
            return Err("emit bytes not deterministic".to_owned());
        }
    }
    Ok(())
}

#[test]
fn formatting_stability() -> Result<(), String> {
    let compact = "version: velvet-ballistics/v1\nname: stable_source\nwhen: {manual: {}}\nsteps:\n  - id: done\n    finish: {result: 7}\n";
    let annotated = "# same semantic workflow\nversion: velvet-ballistics/v1\nname: stable_source\nwhen:\n  manual: {}\nsteps:\n  # not part of artifact\n  - id: done\n    finish:\n      result: 7\n";
    let compact_w = compile(compact)?;
    let annotated_w = compile(annotated)?;
    if compact_w.digest() != annotated_w.digest() {
        return Err("compiled digest changed by formatting".to_owned());
    }
    if compact_w.semantic_source_identity() != annotated_w.semantic_source_identity() {
        return Err("semantic identity changed by formatting".to_owned());
    }
    let compact_e = emit_compiled_artifact(&compact_w).map_err(|e| format!("emit: {e:?}"))?;
    let annotated_e = emit_compiled_artifact(&annotated_w).map_err(|e| format!("emit: {e:?}"))?;
    if compact_e != annotated_e {
        return Err("emit bytes changed by formatting".to_owned());
    }
    if compute_compiled_digest(compact.as_bytes()) == compute_compiled_digest(annotated.as_bytes())
    {
        return Err("source digest must distinguish raw formatting".to_owned());
    }
    Ok(())
}

#[test]
fn alias_parity_run_save() -> Result<(), String> {
    let set = family_source("set");
    let save = family_source("save");
    let set_w = compile(&set)?;
    let save_w = compile(&save)?;
    if set_w.digest() != save_w.digest() {
        return Err("save vs set digest".to_owned());
    }
    if set_w.semantic_source_identity() != save_w.semantic_source_identity() {
        return Err("save vs set semantic".to_owned());
    }
    let set_e = emit_compiled_artifact(&set_w).map_err(|e| format!("emit: {e:?}"))?;
    let save_e = emit_compiled_artifact(&save_w).map_err(|e| format!("emit: {e:?}"))?;
    if set_e != save_e {
        return Err("save vs set emit".to_owned());
    }
    let do_src = family_source("do");
    let run_src = family_source("run");
    let do_w = compile(&do_src)?;
    let run_w = compile(&run_src)?;
    if do_w.digest() != run_w.digest() {
        return Err("run vs do digest".to_owned());
    }
    if do_w.semantic_source_identity() != run_w.semantic_source_identity() {
        return Err("run vs do semantic".to_owned());
    }
    let do_e = emit_compiled_artifact(&do_w).map_err(|e| format!("emit: {e:?}"))?;
    let run_e = emit_compiled_artifact(&run_w).map_err(|e| format!("emit: {e:?}"))?;
    if do_e != run_e {
        return Err("run vs do emit".to_owned());
    }
    Ok(())
}

#[test]
fn complete_artifact_emission_all_families() -> Result<(), String> {
    for family in FAMILIES {
        let source = family_source(family);
        let workflow = compile(&source).map_err(|e| format!("{family}: {e}"))?;
        assert_terminal(&workflow).map_err(|e| format!("{family}: {e}"))?;
        assert_emit_roundtrip(&workflow).map_err(|e| format!("{family}: {e}"))?;
        if family != "do" && family != "run" {
            compile_verified_artifact(source.as_bytes(), &[])
                .map_err(|e| format!("{family} verified: {e:?}"))?;
        } else {
            use vb_compile::verify_compiled_artifact;
            use vb_core::action::{
                ActionContract, ActionName, Idempotency, RetrySafety, SideEffect,
            };
            let contract = ActionContract {
                id: vb_core::ActionId::new(0),
                name: ActionName::new("act_0").map_err(|e| format!("action name: {e:?}"))?,
                input_slot_count: 1,
                output_slot_count: 1,
                max_input_bytes: 64,
                max_output_bytes: 64,
                timeout_ms: 1000,
                idempotency: Idempotency::DeterministicPure,
                side_effect: SideEffect::None,
                retry_safety: RetrySafety::Safe,
                required_capabilities: Box::new([]),
            };
            verify_compiled_artifact(&workflow, &[contract])
                .map_err(|e| format!("{family} verify: {e:?}"))?;
        }
    }
    Ok(())
}
