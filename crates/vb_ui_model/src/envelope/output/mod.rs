//! OutputEnvelope — the main structured output envelope type.

extern crate alloc;

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::envelope::error::EnvelopeError;
use crate::envelope::types::{
    DiagnosticEntry, EnvelopeKind, MAX_DIAGNOSTIC_ENTRIES, MetadataEnvelope, PayloadEnvelope,
    SchemaVersion,
};

/// A structured output envelope with schema version, kind, metadata,
/// optional data payload, and optional diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutputEnvelope {
    /// The schema version of this envelope.
    pub schema_version: SchemaVersion,
    /// The kind of envelope (Success, Error, DiagnosticReport, etc.).
    pub kind: EnvelopeKind,
    /// Metadata for this envelope (run_id, command, timestamp).
    pub metadata: MetadataEnvelope,
    /// Data payload for kinds that use `data` field (Success, Error, Status, Event, Workflow).
    /// Must be `None` for DiagnosticReport kind.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<PayloadEnvelope>,
    /// Diagnostics for DiagnosticReport kind.
    /// Must be empty for all other kinds.
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<DiagnosticEntry>,
}

impl OutputEnvelope {
    /// Returns the schema version.
    pub const fn schema_version(&self) -> &SchemaVersion {
        &self.schema_version
    }

    /// Returns the envelope kind.
    pub const fn kind(&self) -> &EnvelopeKind {
        &self.kind
    }

    /// Returns an optional reference to the data payload.
    pub const fn payload(&self) -> Option<&PayloadEnvelope> {
        self.data.as_ref()
    }

    /// Returns the first diagnostic entry, if any.
    pub fn diagnostic(&self) -> Option<&DiagnosticEntry> {
        self.diagnostics.first()
    }

    /// Creates a new output envelope with the given data payload.
    ///
    /// # Invariants (I5 - Payload invariant)
    /// - `data` contains exactly the typed payload for `kind`
    /// - `DiagnosticReport` kind uses `diagnostics` instead of `data`
    /// - For `DiagnosticReport`: `data` must be `None` and `diagnostics` must be non-empty
    /// - For other kinds: `diagnostics` must be empty
    pub fn new(
        schema_version: SchemaVersion,
        kind: EnvelopeKind,
        metadata: MetadataEnvelope,
        data: Option<PayloadEnvelope>,
        diagnostics: Vec<DiagnosticEntry>,
    ) -> Result<Self, EnvelopeError> {
        if kind == EnvelopeKind::DiagnosticReport {
            if data.is_some() {
                return Err(EnvelopeError::DataFieldNotAllowedForDiagnosticReport);
            }
            if diagnostics.is_empty() {
                return Err(EnvelopeError::DiagnosticReportMustHaveDiagnostics);
            }
            if diagnostics.len() > MAX_DIAGNOSTIC_ENTRIES {
                return Err(EnvelopeError::DiagnosticLimitExceeded {
                    len: diagnostics.len(),
                    max: MAX_DIAGNOSTIC_ENTRIES,
                });
            }
        } else {
            if !diagnostics.is_empty() {
                return Err(EnvelopeError::DiagnosticLimitExceeded {
                    len: diagnostics.len(),
                    max: 0,
                });
            }
        }

        Ok(Self {
            schema_version,
            kind,
            metadata,
            data,
            diagnostics,
        })
    }

    /// Creates a new output envelope for a successful operation.
    ///
    /// Convenience constructor that sets `kind` to `Success` and ensures
    /// no diagnostics are present.
    pub fn success(
        schema_version: SchemaVersion,
        metadata: MetadataEnvelope,
        data: PayloadEnvelope,
    ) -> Result<Self, EnvelopeError> {
        Self::new(
            schema_version,
            EnvelopeKind::Success,
            metadata,
            Some(data),
            Vec::new(),
        )
    }

    /// Creates a new output envelope for a DiagnosticReport.
    ///
    /// Convenience constructor that sets `kind` to `DiagnosticReport` and ensures
    /// data is None and diagnostics is non-empty.
    pub fn diagnostic_report(
        schema_version: SchemaVersion,
        metadata: MetadataEnvelope,
        diagnostics: Vec<DiagnosticEntry>,
    ) -> Result<Self, EnvelopeError> {
        Self::new(
            schema_version,
            EnvelopeKind::DiagnosticReport,
            metadata,
            None,
            diagnostics,
        )
    }
}

#[cfg(test)]
mod tests;
