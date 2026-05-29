#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-022 implementation target: vb_storage::JournalError::SequenceGap from replay ordering.
verus! { pub open spec fn sequence_gap(expected:int,actual:int)->bool { expected != actual } pub proof fn po_vb_7m21_022(expected:int,actual:int) requires expected!=actual ensures sequence_gap(expected,actual) {} }
