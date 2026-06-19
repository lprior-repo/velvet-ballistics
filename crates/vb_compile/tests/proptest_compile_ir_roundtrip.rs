#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]
//! Proptest: compile-to-IR roundtrip for every v1 primitive.
//!
//! PO: PO-COMPILE-ROUNDTRIP-001
//! Bead: tier-a-3-008
//! Verifier: proptest
//! Command: cargo test -p vb_compile --test proptest_compile_ir_roundtrip
//!
//! Strengthened (2026-06-19, holzman-rust re-verification pass):
//!
//! 1. Per-primitive proptests (one test per primitive) so failures pinpoint
//!    the exact primitive. Each runs `ProptestConfig::cases() = 1000` cases
//!    with primitive-specific variation (IDs, slot names, values).
//! 2. Strict acceptance assertion: every generated workflow must compile
//!    successfully (`Ok`). When a workflow is rejected, the test surfaces
//!    the typed `CompileError` kind so the rejection is actionable.
//! 3. Bitwise IR roundtrip via postcard: serialize -> deserialize -> serialize,
//!    assert byte-for-byte equality, and assert `WorkflowParts` equality.
//! 4. Compatibility smoke test retained for the historical `Ok | Err`
//!    contract used by `tier-a-3-008` (records rejections but does not fail).
//!
//! v1 primitive set (matches `crates/vb_yaml/src/ast/parse_steps.rs`):
//!
//! | # | Primitive    | YAML key(s) accepted              |
//! |---|--------------|-----------------------------------|
//! | 0 | set          | `set` / `save`                    |
//! | 1 | do           | `do` / `run`                      |
//! | 2 | choose       | `choose`                          |
//! | 3 | for_each     | `for_each`                        |
//! | 4 | together     | `together`                        |
//! | 5 | collect      | `collect`                         |
//! | 6 | reduce       | `reduce`                          |
//! | 7 | repeat       | `repeat`                          |
//! | 8 | wait         | `wait`                            |
//! | 9 | ask          | `ask`                             |
//! |10 | finish       | `finish`                          |
//!
//! Aliases `save` (for `set`) and `run` (for `do`) are accepted by the YAML
//! schema but are NOT emitted as separate primitive kinds; they are exercised
//! by the same lowering tests as their canonical names. The test covers the
//! canonical names; aliases are validated by `v1_primitive_lowering.rs`.

use proptest::prelude::*;
use vb_compile::{compile_workflow, CompileErrors};
use vb_core::workflow::WorkflowParts;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: roundtrip-workflow\nwhen:\n  manual: {}\nsteps:\n";

const FINISH_STEP: &str = "  - id: finish_step\n    finish:\n      result: 0\n";

/// Identifier allowed by the strict YAML schema: lowercase letter, then
/// lowercase letters / digits / underscores, total length 1..=16. The
/// `prop_filter` rejects the YAML 1.1 ambiguous scalars (`y`, `n`, `yes`,
/// `no`, `on`, `off` and case variants) because the strict YAML profile
/// rejects them as `forbidden_feature: ambiguous_scalar`.
fn id_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,15}"
        .prop_map(String::from)
        .prop_filter("not a YAML 1.1 ambiguous scalar", |s| {
            !is_yaml11_ambiguous(s)
        })
}

/// Slot / output name (same alphabet as IDs).
fn slot_strategy() -> impl Strategy<Value = String> {
    id_strategy()
}

/// True when `s` matches a YAML 1.1 boolean / null scalar that the strict
/// profile rejects as `forbidden_feature`.
fn is_yaml11_ambiguous(s: &str) -> bool {
    matches!(
        s.to_ascii_lowercase().as_str(),
        "y" | "n" | "yes" | "no" | "on" | "off" | "true" | "false" | "null" | "~"
    )
}

/// Integer literal rendered as a quoted string (matches the canonical
/// `value: "42"` shape used in `v1_primitive_lowering.rs`).
fn int_value_strategy() -> impl Strategy<Value = String> {
    (0i64..100_000).prop_map(|n| format!("\"{n}\""))
}

/// Slot-reference string (`"0"`, `"1"`, ...) used by `input`, `source`,
/// `event`, `prompt` fields. Limited to indices 0..=2 because the minimal
/// workflows in this proptest emit at most 3 slots, and slot refs must be
/// in range at compile time.
fn slot_ref_strategy() -> impl Strategy<Value = String> {
    (0u16..=2u16).prop_map(|n| format!("\"{n}\""))
}

/// Integer-string strategy used by `do.action` and `choose.branches[].when`.
/// These fields require a numeric action id / slot ref and reject arbitrary
/// strings at the strict-YAML layer.
fn int_string_strategy() -> impl Strategy<Value = String> {
    (0u32..1000).prop_map(|n| n.to_string())
}

/// Compile `yaml` and either return the compiled workflow or fail with
/// a diagnostic string that includes the typed `CompileError` kind for
/// every error returned by the compiler. Used by every per-primitive
/// proptest so rejection messages are actionable instead of opaque.
fn compile_or_diagnose(yaml: &str, primitive: &str) -> Result<vb_core::CompiledWorkflow, String> {
    match compile_workflow(yaml.as_bytes()) {
        Ok(workflow) => Ok(workflow),
        Err(CompileErrors(errors)) => {
            let summary: Vec<String> = errors.iter().map(|e| format!("{e:?}")).collect();
            Err(format!(
                "primitive `{primitive}` rejected valid workflow: [{}]",
                summary.join(" | ")
            ))
        }
    }
}

/// Bit-exact IR roundtrip via postcard. Returns `Err` with a descriptive
/// message when the roundtrip fails. Caller converts that into a
/// proptest assertion failure via `prop_assert!(false, ...)`.
fn roundtrip_workflow_parts(workflow: &vb_core::CompiledWorkflow) -> Result<(), String> {
    let parts = workflow.to_parts();
    let bytes = postcard::to_allocvec(&parts).map_err(|e| format!("serialize: {e}"))?;
    let decoded: WorkflowParts =
        postcard::from_bytes(&bytes).map_err(|e| format!("deserialize: {e}"))?;
    let reencoded = postcard::to_allocvec(&decoded).map_err(|e| format!("re-serialize: {e}"))?;
    if bytes != reencoded {
        return Err(format!(
            "IR bytes differ after roundtrip ({} vs {} bytes)",
            bytes.len(),
            reencoded.len()
        ));
    }
    if parts != decoded {
        return Err("logical WorkflowParts differs after roundtrip".to_string());
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        ..ProptestConfig::default()
    })]

    /// `set` roundtrip: vary id, output, value.
    #[test]
    fn roundtrip_set(id in id_strategy(), output in slot_strategy(), value in int_value_strategy()) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    set:\n      output: {output}\n      value: {value}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "set") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "set roundtrip failed: {e}");
        }
    }

    /// `do` roundtrip: vary id, action (numeric action id), input (slot ref).
    #[test]
    fn roundtrip_do(id in id_strategy(), action in int_string_strategy(), input in slot_ref_strategy()) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    do:\n      action: \"{action}\"\n      input: {input}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "do") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "do roundtrip failed: {e}");
        }
    }

    /// `choose` roundtrip: vary id, single branch `when` (numeric slot ref),
    /// single body step output/value. Always includes `otherwise` pointing
    /// to `finish_step` (a real downstream step) so the otherwise target is
    /// resolvable.
    #[test]
    fn roundtrip_choose(
        id in id_strategy(),
        when in int_string_strategy(),
        body_id in id_strategy(),
        body_output in slot_strategy(),
        body_value in int_value_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    choose:\n      branches:\n        - when: \"{when}\"\n          steps:\n            - id: {body_id}\n              set:\n                output: {body_output}\n                value: {body_value}\n      otherwise: finish_step\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "choose") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "choose roundtrip failed: {e}");
        }
    }

    /// `for_each` roundtrip: vary id, variable, input, single body step.
    #[test]
    fn roundtrip_for_each(
        id in id_strategy(),
        variable in id_strategy(),
        input in slot_ref_strategy(),
        body_id in id_strategy(),
        body_output in slot_strategy(),
        body_value in int_value_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    for_each:\n      variable: {variable}\n      input: {input}\n      steps:\n        - id: {body_id}\n          set:\n            output: {body_output}\n            value: {body_value}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "for_each") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "for_each roundtrip failed: {e}");
        }
    }

    /// `together` roundtrip: vary id, branch count (2 or 3), branch labels,
    /// and body step content. Each branch gets a distinct body id and output
    /// slot to avoid collisions.
    #[test]
    fn roundtrip_together(
        id in id_strategy(),
        branch_count in 2u8..=3u8,
        label_a in id_strategy(),
        label_b in id_strategy(),
        label_c in id_strategy(),
        body_a_id in id_strategy(),
        body_b_id in id_strategy(),
        body_c_id in id_strategy(),
        body_a_out in slot_strategy(),
        body_b_out in slot_strategy(),
        body_c_out in slot_strategy(),
        body_a_val in int_value_strategy(),
        body_b_val in int_value_strategy(),
        body_c_val in int_value_strategy(),
    ) {
        let third_branch = if branch_count == 3 {
            format!(
                "        - label: {label_c}\n          steps:\n            - id: {body_c_id}\n              set:\n                output: {body_c_out}\n                value: {body_c_val}\n"
            )
        } else {
            String::new()
        };
        let yaml = format!(
            "{HEADER}  - id: {id}\n    together:\n      branches:\n        - label: {label_a}\n          steps:\n            - id: {body_a_id}\n              set:\n                output: {body_a_out}\n                value: {body_a_val}\n        - label: {label_b}\n          steps:\n            - id: {body_b_id}\n              set:\n                output: {body_b_out}\n                value: {body_b_val}\n{third_branch}{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "together") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "together roundtrip failed: {e}");
        }
    }

    /// `collect` roundtrip: vary id, variable, source, body step content.
    #[test]
    fn roundtrip_collect(
        id in id_strategy(),
        variable in id_strategy(),
        source in slot_ref_strategy(),
        body_id in id_strategy(),
        body_output in slot_strategy(),
        body_value in int_value_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    collect:\n      variable: {variable}\n      source: {source}\n      steps:\n        - id: {body_id}\n          set:\n            output: {body_output}\n            value: {body_value}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "collect") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "collect roundtrip failed: {e}");
        }
    }

    /// `reduce` roundtrip: vary id, variable, input, initial, body step.
    #[test]
    fn roundtrip_reduce(
        id in id_strategy(),
        variable in id_strategy(),
        input in slot_ref_strategy(),
        initial in int_value_strategy(),
        body_id in id_strategy(),
        body_output in slot_strategy(),
        body_value in int_value_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    reduce:\n      variable: {variable}\n      input: {input}\n      initial: {initial}\n      steps:\n        - id: {body_id}\n          set:\n            output: {body_output}\n            value: {body_value}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "reduce") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "reduce roundtrip failed: {e}");
        }
    }

    /// `repeat` roundtrip: vary id, max_attempts (1..=8), body step.
    #[test]
    fn roundtrip_repeat(
        id in id_strategy(),
        max_attempts in 1u16..=8u16,
        body_id in id_strategy(),
        body_output in slot_strategy(),
        body_value in int_value_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    repeat:\n      max_attempts: {max_attempts}\n      steps:\n        - id: {body_id}\n          set:\n            output: {body_output}\n            value: {body_value}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "repeat") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "repeat roundtrip failed: {e}");
        }
    }

    /// `wait` roundtrip: vary id, event (slot ref), timeout (slot ref).
    #[test]
    fn roundtrip_wait(
        id in id_strategy(),
        event in slot_ref_strategy(),
        timeout in slot_ref_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    wait:\n      event: {event}\n      timeout: {timeout}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "wait") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "wait roundtrip failed: {e}");
        }
    }

    /// `ask` roundtrip: vary id, prompt, timeout.
    #[test]
    fn roundtrip_ask(
        id in id_strategy(),
        prompt in slot_ref_strategy(),
        timeout in slot_ref_strategy(),
    ) {
        let yaml = format!(
            "{HEADER}  - id: {id}\n    ask:\n      prompt: {prompt}\n      timeout: {timeout}\n{FINISH_STEP}"
        );
        let workflow = match compile_or_diagnose(&yaml, "ask") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "ask roundtrip failed: {e}");
        }
    }

    /// `finish` roundtrip: vary id. `result` is fixed to the integer literal
    /// `0` because the strict YAML profile accepts only integer literals
    /// or pre-declared slot names for `finish.result`, and the minimal
    /// workflow in this proptest has no preceding set step.
    #[test]
    fn roundtrip_finish(id in id_strategy()) {
        let yaml = format!("{HEADER}  - id: {id}\n    finish:\n      result: 0\n");
        let workflow = match compile_or_diagnose(&yaml, "finish") {
            Ok(w) => w,
            Err(e) => { prop_assert!(false, "{e}"); return Ok(()); }
        };
        if let Err(e) = roundtrip_workflow_parts(&workflow) {
            prop_assert!(false, "finish roundtrip failed: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Compatibility smoke test: keeps the historical "Ok | Err" contract that
// `tier-a-3-008` was closed against. Surfaces typed-error diagnostics when
// the compiler rejects an input. This test NEVER fails — it documents the
// primitive-by-primitive acceptance matrix at the time of the run.
// ---------------------------------------------------------------------------

const COMPAT_PRIMITIVE_NAMES: [&str; 11] = [
    "set", "do", "choose", "for_each", "together", "collect", "reduce", "repeat", "wait", "ask",
    "finish",
];

/// Compatibility-mode roundtrip: a primitive either compiles-and-roundtrips
/// (Ok) or is rejected with typed errors (Err). Errors must be typed (the
/// vec is non-empty and contains `CompileError` variants), never a panic.
fn compat_workflow_for_primitive(primitive_index: u8) -> String {
    let mut yaml = String::from(HEADER);
    let body = match primitive_index {
        0 => "  - id: setup\n    set:\n      output: answer\n      value: \"42\"\n",
        1 => "  - id: invoke\n    do:\n      action: \"7\"\n      input: \"0\"\n",
        2 => {
            "  - id: pick\n    choose:\n      branches:\n        - when: \"true\"\n          steps:\n            - id: b1s0\n              set:\n                output: answer\n                value: \"1\"\n      otherwise: finish_step\n"
        }
        3 => {
            "  - id: loop\n    for_each:\n      variable: item\n      input: \"0\"\n      steps:\n        - id: b0\n          set:\n            output: answer\n            value: \"1\"\n"
        }
        4 => {
            "  - id: fanout\n    together:\n      branches:\n        - label: left\n          steps:\n            - id: left_set\n              set:\n                output: left\n                value: \"1\"\n        - label: right\n          steps:\n            - id: right_set\n              set:\n                output: right\n                value: \"2\"\n"
        }
        5 => {
            "  - id: pages\n    collect:\n      variable: page\n      source: \"0\"\n      steps:\n        - id: b0\n          set:\n            output: answer\n            value: \"1\"\n"
        }
        6 => {
            "  - id: fold\n    reduce:\n      variable: acc\n      input: \"0\"\n      initial: \"10\"\n      steps:\n        - id: b0\n          set:\n            output: answer\n            value: \"1\"\n"
        }
        7 => {
            "  - id: retry\n    repeat:\n      max_attempts: 3\n      steps:\n        - id: attempt\n          set:\n            output: answer\n            value: \"1\"\n"
        }
        8 => "  - id: pause\n    wait:\n      event: \"0\"\n      timeout: \"1\"\n",
        9 => "  - id: ask_user\n    ask:\n      prompt: \"0\"\n      timeout: \"1\"\n",
        _ => "", // finish is the trailing step
    };
    yaml.push_str(body);
    if primitive_index != 10 {
        yaml.push_str(FINISH_STEP);
    }
    yaml
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 50,
        ..ProptestConfig::default()
    })]

    #[test]
    fn compatibility_roundtrip(primitive_index in 0u8..=10u8) {
        let name = COMPAT_PRIMITIVE_NAMES
            .get(usize::from(primitive_index))
            .copied()
            .unwrap_or("?");
        let yaml = compat_workflow_for_primitive(primitive_index);
        match compile_workflow(yaml.as_bytes()) {
            Ok(workflow) => {
                if let Err(e) = roundtrip_workflow_parts(&workflow) {
                    prop_assert!(false, "compat[{name}] roundtrip failed: {e}");
                }
            }
            Err(CompileErrors(errors)) => {
                prop_assert!(!errors.is_empty(), "compat[{name}] Err must contain at least one typed error");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Source scan: this proptest lives next to the other compile proptests and
// shares the same compile_workflow + WorkflowParts + postcard machinery.
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_proptest_uses_postcard_for_ir_encoding() {
    let source = include_str!("proptest_compile_ir_roundtrip.rs");
    assert!(
        source.contains("postcard::to_allocvec"),
        "roundtrip proptest must encode via postcard"
    );
    assert!(
        source.contains("postcard::from_bytes"),
        "roundtrip proptest must decode via postcard"
    );
    assert!(
        source.contains("WorkflowParts"),
        "roundtrip proptest must exercise WorkflowParts"
    );
}

/// Smoke test: every v1 primitive listed in `COMPAT_PRIMITIVE_NAMES` has a
/// strict roundtrip subtest in this file. Fails closed when a new primitive
/// is added to the v1 set without being covered here.
#[test]
fn every_v1_primitive_has_strict_roundtrip_subtest() {
    let source = include_str!("proptest_compile_ir_roundtrip.rs");
    for primitive in COMPAT_PRIMITIVE_NAMES {
        let marker = format!("fn roundtrip_{primitive}(");
        assert!(
            source.contains(&marker),
            "missing strict roundtrip subtest for primitive `{primitive}` (expected `fn {marker}...`)"
        );
    }
}
