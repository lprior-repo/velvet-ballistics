#![allow(unused_imports)]
use vstd::prelude::*;
// PO-vb-7m21-006 implementation target: vb_storage::codec::decode_record_header / JournalError::UnsupportedSchemaVersion.
verus! { pub open spec fn unsupported_schema(version:int,current:int)->bool { version > current } pub proof fn po_vb_7m21_006(version:int,current:int) requires version>current ensures unsupported_schema(version,current) {} }
