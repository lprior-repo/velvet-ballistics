#![forbid(unsafe_code)]
//! Lazily-formatted display for [`SlotValue`] that resolves arena handles
//! against a [`ValueStore`]. Allocations are deferred until `Display::fmt`
//! is called (i.e., when `to_string()` or `format!()` is invoked).
//!
//! # Example
//! ```
//! # use vb_core::value::SlotValue;
//! # use vb_core::value_store::ValueStore;
//! # use vb_core::value::SlotValueDisplay;
//! let store = ValueStore::new();
//! let value = SlotValue::Null;
//! let display = SlotValueDisplay::new(&value, &store);
//! assert_eq!(format!("{display}"), "null");
//! ```

use crate::value_store::ValueStore;
use core::fmt;

use super::SlotValue;

/// Lazily-formatted display for [`SlotValue`] that resolves arena handles
/// against a [`ValueStore`]. Allocations are deferred until `Display::fmt`
/// is called (i.e., when `to_string()` or `format!()` is invoked).
#[derive(Debug)]
pub struct SlotValueDisplay<'a>(&'a SlotValue, &'a ValueStore);

impl<'a> SlotValueDisplay<'a> {
    /// Create a new formatter for `value` using `store` for arena resolution.
    #[inline]
    pub fn new(value: &'a SlotValue, store: &'a ValueStore) -> Self {
        Self(value, store)
    }
}

impl fmt::Display for SlotValueDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            SlotValue::Null => write!(f, "null"),
            SlotValue::Bool(v) => write!(f, "{v}"),
            SlotValue::I64(v) => write!(f, "{v}"),
            SlotValue::F64(v) => write!(f, "{v}"),
            SlotValue::Symbol(id) => match self.1.symbol(*id) {
                Ok(s) => write!(f, "symbol:{s}"),
                Err(_) => write!(f, "symbol:{}", id.get()),
            },
            SlotValue::List(id) => match self.1.list(*id) {
                Ok(items) => {
                    write!(f, "[")?;
                    let mut first = true;
                    for item in items {
                        if !first {
                            write!(f, ", ")?;
                        }
                        first = false;
                        SlotValueDisplay::new(item, self.1).fmt(f)?;
                    }
                    write!(f, "]")
                }
                Err(_) => write!(f, "list:{}", id.get()),
            },
            SlotValue::Object(id) => match self.1.object(*id) {
                Ok(fields) => {
                    write!(f, "{{")?;
                    let mut first = true;
                    for field in fields {
                        if !first {
                            write!(f, ", ")?;
                        }
                        first = false;
                        let key_display = match self.1.symbol(field.key) {
                            Ok(s) => s,
                            Err(_) => return write!(f, "{}:", field.key.get()),
                        };
                        write!(f, "{key_display}: ")?;
                        SlotValueDisplay::new(&field.value, self.1).fmt(f)?;
                    }
                    write!(f, "}}")
                }
                Err(_) => write!(f, "object:{}", id.get()),
            },
            SlotValue::Blob(id) => match self.1.blob(*id) {
                Ok(bytes) => write!(f, "blob:<{} bytes>", bytes.len()),
                Err(_) => write!(f, "blob:{}", id.get()),
            },
        }
    }
}
