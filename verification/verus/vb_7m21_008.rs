#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-036 implementation target: vb_storage manifest/keyspace public fixture parity.
verus! { pub open spec fn missing_manifest_keyspace(declared:int,present:int)->bool { declared > present } pub proof fn po_vb_7m21_036(declared:int,present:int) requires declared>present ensures missing_manifest_keyspace(declared,present) {} }
