//! Borrowed lexical environment shared by canonical scalar and action lowering.
//!
//! The scope is a lightweight borrowed chain: each level holds references to
//! the parent HashMaps for inserts that are already visible, and a singly
//! linked `parent` pointer for shadowing/binding inserts. Lookups fall
//! through the chain without copying whole maps — the perf fix for the YAML
//! clone waivers. Only the level that introduces a binding pays for a tiny
//! overlay map (capacity 1); parent maps are never cloned.

use std::collections::HashMap;
use vb_core::SlotIdx;
use vb_expr::expr_typecheck::ExprType;

#[derive(Clone, Copy)]
pub(in crate::mod_compile_lowering) struct CanonicalScope<'a> {
    pub outputs: &'a HashMap<String, SlotIdx>,
    pub step_outputs: &'a HashMap<String, SlotIdx>,
    pub slot_types: &'a HashMap<SlotIdx, ExprType>,
    /// Optional parent scope for chained borrowed lookups. `None` at root.
    /// `Some(parent)` means: check `outputs`/`step_outputs`/`slot_types` first,
    /// then fall back to `parent.get_*` for lexically outer bindings.
    pub parent: Option<&'a CanonicalScope<'a>>,
}

impl<'a> CanonicalScope<'a> {
    /// Look up an output binding, checking local map then parent chain.
    #[inline]
    pub(in crate::mod_compile_lowering) fn get_output(&self, name: &str) -> Option<SlotIdx> {
        self.outputs
            .get(name)
            .copied()
            .or_else(|| self.parent.and_then(|p| p.get_output(name)))
    }

    /// Look up a step binding, checking local map then parent chain.
    #[inline]
    pub(in crate::mod_compile_lowering) fn get_step(&self, name: &str) -> Option<SlotIdx> {
        self.step_outputs
            .get(name)
            .copied()
            .or_else(|| self.parent.and_then(|p| p.get_step(name)))
    }

    /// Look up a slot type, checking local map then parent chain.
    #[inline]
    pub(in crate::mod_compile_lowering) fn get_type(&self, slot: SlotIdx) -> Option<ExprType> {
        self.slot_types
            .get(&slot)
            .copied()
            .or_else(|| self.parent.and_then(|p| p.get_type(slot)))
    }

    #[inline]
    pub(in crate::mod_compile_lowering) fn contains_output(&self, name: &str) -> bool {
        self.get_output(name).is_some()
    }

    #[inline]
    pub(in crate::mod_compile_lowering) fn contains_step(&self, name: &str) -> bool {
        self.get_step(name).is_some()
    }
}

/// Declaration roots are not mutable output aliases, in any lexical scope.
pub(in crate::mod_compile_lowering) fn validate_output_alias(
    output: &str,
) -> Result<(), crate::CompileError> {
    let reserved = output.starts_with('$')
        || ["input.", "vars.", "secrets.", "step.", "steps."]
            .iter()
            .any(|root| output.starts_with(root));
    if reserved {
        return Err(crate::CompileError::InvalidName {
            field: "set.output",
            value: output.into(),
        });
    }
    Ok(())
}
