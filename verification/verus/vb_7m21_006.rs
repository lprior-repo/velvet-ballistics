#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-027 implementation target: vb_storage::JournalError::DuplicateEvent.
verus! { pub open spec fn duplicate_event(existing:bool,identical:bool)->bool { existing && !identical } pub proof fn po_vb_7m21_027(existing:bool,identical:bool) requires existing,!identical ensures duplicate_event(existing,identical) {} }
