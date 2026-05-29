#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-017 implementation target: vb_storage public fixture observes missing side-index typed outcome.
verus! { pub open spec fn index_parity_mismatch(event:bool,index:bool)->bool { event && !index } pub proof fn po_vb_7m21_017(event:bool,index:bool) requires event,!index ensures index_parity_mismatch(event,index) {} }
