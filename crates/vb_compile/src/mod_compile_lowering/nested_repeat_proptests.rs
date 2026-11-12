#![forbid(unsafe_code)]

//! Property tests for nested Repeat / ForEach boundary gates (vb-eirmk.4.5).
//!
//! - `repeat_max_attempts_prop`: `max_attempts == 0` fails with the exact
//!   `repeat.max_attempts` shape; any positive value compiles with nodes.
//! - `for_each_concurrency_prop`: `at_once` is a positive upper bound,
//!   defaulting to 1; zero fails with the exact `for_each.at_once` shape.

use crate::{CompileError, compile_workflow};
use proptest::prelude::*;

const HEADER: &str =
    "version: velvet-ballistics/v1\nname: nested_repeat_prop\nwhen: {manual: {}}\n";

fn repeat_source(attempts: u16) -> String {
    format!(
        "{HEADER}steps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: bounded\n    repeat:\n      max_attempts: {attempts}\n      steps:\n        - id: inner\n          set: {{output: tmp, value: '7'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn for_each_source(at_once: Option<u32>) -> String {
    let at_once = at_once.map_or_else(String::new, |v| format!("      at_once: {v}\n"));
    format!(
        "{HEADER}inputs:\n  items: {{is: list, of: any, max: 4}}\nsteps:\n  - id: seed\n    set: {{output: seed, value: '42'}}\n  - id: outer\n    for_each:\n      input: '$input.items'\n      variable: entry\n{at_once}      steps:\n        - id: leaf\n          set: {{output: picked, value: '$entry'}}\n  - id: done\n    finish: {{result: '$seed'}}\n"
    )
}

fn at_once_rejection(errors: &crate::CompileErrors) -> bool {
    errors.0.iter().any(|e| {
        matches!(
            e,
            CompileError::StepFieldShape {
                field: "for_each.at_once",
                expected: "a positive concurrency upper bound",
                ..
            }
        )
    })
}

proptest! {
    #[test]
    fn repeat_max_attempts_prop(attempts in 0..=3u16) {
        let result = compile_workflow(repeat_source(attempts).as_bytes());
        if attempts == 0 {
            let errors = result.expect_err("max_attempts 0 must fail");
            prop_assert!(
                errors.0.iter().any(|e| matches!(
                    e,
                    CompileError::StepFieldShape {
                        field: "repeat.max_attempts",
                        expected: "a value of at least 1 (got 0)",
                        ..
                    }
                )),
                "max_attempts 0 wrong diagnostic: {errors:?}"
            );
        } else {
            match result {
                Ok(workflow) => prop_assert!(workflow.node_count() > 0),
                Err(errors) => return Err(proptest::test_runner::TestCaseError::fail(format!(
                    "max_attempts {attempts} must compile: {errors:?}"
                ))),
            }
        }
    }

    #[test]
    fn for_each_concurrency_prop(at_once in prop_oneof![
        Just(None),
        Just(Some(0u32)),
        Just(Some(1u32)),
        Just(Some(2u32)),
        Just(Some(u32::MAX)),
        any::<u32>().prop_map(Some),
    ]) {
        let result = compile_workflow(for_each_source(at_once).as_bytes());
        match at_once {
            value if value != Some(0) => match result {
                Ok(workflow) => prop_assert!(workflow.node_count() > 0),
                Err(errors) => return Err(proptest::test_runner::TestCaseError::fail(format!(
                    "at_once {at_once:?} must compile: {errors:?}"
                ))),
            },
            _ => {
                let errors = result.expect_err("zero at_once must fail");
                prop_assert!(
                    at_once_rejection(&errors),
                    "at_once {at_once:?} wrong diagnostic: {errors:?}"
                );
            }
        }
    }
}
