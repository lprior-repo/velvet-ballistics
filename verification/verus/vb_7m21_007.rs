#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-031 implementation target: vb_storage::recovery snapshot-plus-tail replay relation.
verus! { pub open spec fn replay_tail(snapshot:int,tail:int,valid:bool)->bool { valid && snapshot < tail } pub proof fn po_vb_7m21_031(snapshot:int,tail:int) requires snapshot<tail ensures replay_tail(snapshot,tail,true) {} }
