// Verus proof obligations for cancel/kill lifecycle lattice.
//
// Registry: PO-VERUS cancel/kill lattice.
// Production mirror: `extern_cancel_kill_lattice.rs` ->
// `production_inner/cancel_kill_lattice_production.rs`, bound to
// `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-174`.

use vstd::prelude::*;

verus! {

#[path = "extern_cancel_kill_lattice.rs"]
mod production;

pub open spec fn live(s: production::Shard) -> bool {
    s.run_state_present
}

pub open spec fn terminal(s: production::Shard) -> bool {
    s.terminal_runs_present
}

pub open spec fn command_is_allowed(s: production::Shard) -> bool {
    live(s) || terminal(s)
}

pub open spec fn next_live(s: production::Shard) -> bool {
    if command_is_allowed(s) { false } else { s.run_state_present }
}

pub open spec fn next_terminal(s: production::Shard) -> bool {
    if command_is_allowed(s) { true } else { s.terminal_runs_present }
}

pub open spec fn next_pending_timer(s: production::Shard) -> bool {
    if live(s) { false } else { s.pending_timer_present }
}

pub open spec fn next_terminal_event(s: production::Shard) -> bool {
    if live(s) { true } else { s.terminal_event_emitted }
}

pub open spec fn next_stale_authority(s: production::Shard) -> bool {
    if command_is_allowed(s) { false } else { s.stale_authority_valid }
}

pub open spec fn lifecycle_invariant(s: production::Shard) -> bool {
    &&& s.terminal_event_emitted ==> s.terminal_runs_present
    &&& s.terminal_runs_present ==> !s.run_state_present
}

pub assume_specification[ production::Shard::handle_cancel ](
    shard: &mut production::Shard,
    run: production::RunId,
    reason: Option<production::String>,
) -> (r: production::RuntimeResult<()>)
    ensures
        match r {
            Ok(()) => command_is_allowed(*old(shard)),
            Err(production::RuntimeError::RunNotFound) => !command_is_allowed(*old(shard)),
        },
        final(shard).run_state_present == next_live(*old(shard)),
        final(shard).terminal_runs_present == next_terminal(*old(shard)),
        final(shard).pending_timer_present == next_pending_timer(*old(shard)),
        final(shard).terminal_event_emitted == next_terminal_event(*old(shard)),
        final(shard).stale_authority_valid == next_stale_authority(*old(shard)),
;

pub assume_specification[ production::Shard::handle_kill ](
    shard: &mut production::Shard,
    run: production::RunId,
    reason: Option<production::String>,
) -> (r: production::RuntimeResult<()>)
    ensures
        match r {
            Ok(()) => command_is_allowed(*old(shard)),
            Err(production::RuntimeError::RunNotFound) => !command_is_allowed(*old(shard)),
        },
        final(shard).run_state_present == next_live(*old(shard)),
        final(shard).terminal_runs_present == next_terminal(*old(shard)),
        final(shard).pending_timer_present == next_pending_timer(*old(shard)),
        final(shard).terminal_event_emitted == next_terminal_event(*old(shard)),
        final(shard).stale_authority_valid == next_stale_authority(*old(shard)),
;

pub exec fn wrapper_cancel(
    shard: &mut production::Shard,
    run: production::RunId,
) -> (r: production::RuntimeResult<()>)
    ensures
        final(shard).run_state_present == next_live(*old(shard)),
        final(shard).terminal_runs_present == next_terminal(*old(shard)),
        final(shard).terminal_event_emitted == next_terminal_event(*old(shard)),
        final(shard).stale_authority_valid == next_stale_authority(*old(shard)),
{
    shard.handle_cancel(run, None)
}

pub exec fn wrapper_kill(
    shard: &mut production::Shard,
    run: production::RunId,
) -> (r: production::RuntimeResult<()>)
    ensures
        final(shard).run_state_present == next_live(*old(shard)),
        final(shard).terminal_runs_present == next_terminal(*old(shard)),
        final(shard).terminal_event_emitted == next_terminal_event(*old(shard)),
        final(shard).stale_authority_valid == next_stale_authority(*old(shard)),
{
    shard.handle_kill(run, None)
}

pub proof fn proof_cancel_kill_live_only(s: production::Shard)
    ensures
        command_is_allowed(s) == (live(s) || terminal(s)),
        !command_is_allowed(s) ==> !live(s) && !terminal(s),
{
    assert(command_is_allowed(s) == (live(s) || terminal(s))) by (compute);
    assert(!command_is_allowed(s) ==> !live(s) && !terminal(s)) by (compute);
}

pub proof fn proof_cancel_from_live_emits_one_terminal(s: production::Shard)
    requires
        live(s),
        !terminal(s),
        !s.terminal_event_emitted,
    ensures
        next_live(s) == false,
        next_terminal(s) == true,
        next_terminal_event(s) == true,
        next_stale_authority(s) == false,
        next_pending_timer(s) == false,
{
    assert(next_live(s) == false) by (compute);
    assert(next_terminal(s) == true) by (compute);
    assert(next_terminal_event(s) == true) by (compute);
    assert(next_stale_authority(s) == false) by (compute);
    assert(next_pending_timer(s) == false) by (compute);
}

pub proof fn proof_terminal_command_does_not_emit_second_terminal(s: production::Shard)
    requires
        !live(s),
        terminal(s),
        s.terminal_event_emitted,
    ensures
        next_live(s) == false,
        next_terminal(s) == true,
        next_terminal_event(s) == s.terminal_event_emitted,
        next_stale_authority(s) == false,
{
    assert(next_live(s) == false) by (compute);
    assert(next_terminal(s) == true) by (compute);
    assert(next_terminal_event(s) == s.terminal_event_emitted) by (compute);
    assert(next_stale_authority(s) == false) by (compute);
}

pub proof fn proof_missing_run_rejected_without_state_change(s: production::Shard)
    requires
        !live(s),
        !terminal(s),
    ensures
        !command_is_allowed(s),
        next_live(s) == s.run_state_present,
        next_terminal(s) == s.terminal_runs_present,
        next_terminal_event(s) == s.terminal_event_emitted,
        next_stale_authority(s) == s.stale_authority_valid,
{
    assert(!command_is_allowed(s)) by (compute);
    assert(next_live(s) == s.run_state_present) by (compute);
    assert(next_terminal(s) == s.terminal_runs_present) by (compute);
    assert(next_terminal_event(s) == s.terminal_event_emitted) by (compute);
    assert(next_stale_authority(s) == s.stale_authority_valid) by (compute);
}

pub proof fn proof_cancel_then_kill_single_terminal_winner(s: production::Shard)
    requires
        live(s),
        !terminal(s),
        !s.terminal_event_emitted,
    ensures
        next_terminal_event(s),
        next_terminal_event(production::Shard {
            run_state_present: next_live(s),
            terminal_runs_present: next_terminal(s),
            pending_timer_present: next_pending_timer(s),
            terminal_event_emitted: next_terminal_event(s),
            stale_authority_valid: next_stale_authority(s),
            counters: s.counters,
            trace_ring: s.trace_ring,
        }) == next_terminal_event(s),
{
    let after_cancel = production::Shard {
        run_state_present: next_live(s),
        terminal_runs_present: next_terminal(s),
        pending_timer_present: next_pending_timer(s),
        terminal_event_emitted: next_terminal_event(s),
        stale_authority_valid: next_stale_authority(s),
        counters: s.counters,
        trace_ring: s.trace_ring,
    };
    assert(next_terminal_event(s)) by (compute);
    assert(next_terminal_event(after_cancel) == next_terminal_event(s)) by (compute);
}

fn main() {}

} // verus!
