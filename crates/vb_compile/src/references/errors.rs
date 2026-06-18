//! Maps shared-validator errors into compile-specific diagnostics.
//!
//! Translates `vb_validate::ValidationError` variants into `CompileError`
//! with appropriate kind annotations for the reference root.

use crate::CompileError;
use vb_validate::ValidationError;

/// Maps a `vb_validate::ValidationError` from shared reference validation into
/// a `CompileError` with source-location context.
pub(super) fn map_validation_error(reference: &str, error: &ValidationError) -> CompileError {
    match error {
        ValidationError::SecretNotDeclared { secret } => {
            // The shared vb_validate layer reports undeclared secret references
            // as `SecretNotDeclared { secret }`. The compile layer surfaces
            // these to users as `UnknownReferenceName { kind: "secrets", .. }`
            // so the error type and reference-kind reporting are uniform with
            // other unknown-reference cases.
            CompileError::UnknownReferenceName {
                kind: "secrets",
                reference: Box::from(reference),
                name: Box::from(secret.as_str()),
            }
        }
        ValidationError::UnknownReference { .. } => {
            let Some(body) = reference.strip_prefix('$') else {
                return CompileError::UnknownReferenceRoot {
                    reference: Box::from(reference),
                    root: Box::from(reference),
                };
            };
            let Some((root, tail)) = body.split_once('.') else {
                return CompileError::UnknownReferenceRoot {
                    reference: Box::from(reference),
                    root: Box::from(body),
                };
            };
            let name = match tail.split_once('.') {
                Some((name, _)) => name,
                None => tail,
            };
            let kind = match root {
                "input" => "input",
                "var" | "vars" => "var",
                "secrets" => "secrets",
                "step" | "steps" => "step",
                _ => {
                    return CompileError::UnknownReferenceRoot {
                        reference: Box::from(reference),
                        root: Box::from(root),
                    };
                }
            };
            CompileError::UnknownReferenceName {
                kind,
                reference: Box::from(reference),
                name: Box::from(name),
            }
        }
        _ => CompileError::IllegalReference {
            reference: Box::from(reference),
        },
    }
}
