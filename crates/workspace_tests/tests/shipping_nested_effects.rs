#![forbid(unsafe_code)]
//! Three-level nested effect mixtures on owned public admission.
//!
//! Bead: vb-eirmk.4.7.3. Cells: ForEach > Repeat > Ask,
//! Repeat > Choose > Wait, Together > Choose > Do, scoped retry/handler, and
//! the retained NestedTogether prohibition. Existing registered pair/matrix
//! suites stay untouched; this target adds only the three-level owned
//! source -> accepted artifact -> Runtime -> Fjall cold-reopen behavior.

#[path = "vb_f2_nested_matrix/source.rs"]
mod source;

#[path = "shipping_nested_effects/support.rs"]
mod support;

#[path = "shipping_nested_effects/foreach_repeat_ask.rs"]
mod foreach_repeat_ask;

#[path = "shipping_nested_effects/repeat_choose_wait.rs"]
mod repeat_choose_wait;

#[path = "shipping_nested_effects/together_choose_do.rs"]
mod together_choose_do;

#[path = "shipping_nested_effects/nested_together.rs"]
mod nested_together;

#[path = "shipping_nested_effects/repeat_retry_handler.rs"]
mod repeat_retry_handler;
