use super::RuntimeError;

mod admission;
mod core;
mod unit;

impl PartialEq for RuntimeError {
    fn eq(&self, other: &Self) -> bool {
        unit::runtime_error_unit_eq(self, other)
            || core::runtime_error_core_field_eq(self, other)
            || admission::runtime_error_admission_field_eq(self, other)
    }
}

impl Eq for RuntimeError {}
