// SPDX-License-Identifier: AGPL-3.0-or-later
//! The structured outcome of validation: a list of [`Violation`]s with paths.
//!
//! Validation never returns a bare boolean. It returns a [`ValidationReport`] of
//! addressable violations so that callers - the commit path, the CLI, and the
//! future MCP/LLM layer - can act on, display, or feed each one back for
//! correction. Paths use the canonical openEHR `attribute[node_id]` syntax.

use serde::{Deserialize, Serialize};

/// How serious a violation is. Only [`Severity::Error`] blocks a commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// A breach of the Reference Model or template that blocks the commit.
    Error,
    /// A non-blocking advisory (e.g. a recommended node is absent).
    Warning,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

/// A single validation finding, addressable by its RM path.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Violation {
    pub severity: Severity,
    /// The canonical openEHR path to the offending node, e.g.
    /// `/content[openEHR-EHR-OBSERVATION.blood_pressure.v2]/.../value/magnitude`.
    pub rm_path: String,
    /// The constraint or rule that was breached, e.g. `C_DV_QUANTITY` or
    /// `RM:element_value_or_null`.
    pub constraint: String,
    /// A human-readable explanation.
    pub message: String,
}

/// The result of validating a Composition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub violations: Vec<Violation>,
}

impl ValidationReport {
    /// An empty, passing report.
    pub fn new() -> Self {
        Self {
            valid: true,
            violations: Vec::new(),
        }
    }

    /// Record an error, marking the report invalid.
    pub fn error(
        &mut self,
        rm_path: impl Into<String>,
        constraint: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.valid = false;
        self.violations.push(Violation {
            severity: Severity::Error,
            rm_path: rm_path.into(),
            constraint: constraint.into(),
            message: message.into(),
        });
    }

    /// Record a non-blocking warning.
    pub fn warning(
        &mut self,
        rm_path: impl Into<String>,
        constraint: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.violations.push(Violation {
            severity: Severity::Warning,
            rm_path: rm_path.into(),
            constraint: constraint.into(),
            message: message.into(),
        });
    }

    /// Merge another report into this one.
    pub fn merge(&mut self, other: ValidationReport) {
        if !other.valid {
            self.valid = false;
        }
        self.violations.extend(other.violations);
    }

    /// The number of error-severity violations.
    pub fn error_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == Severity::Error)
            .count()
    }

    /// The number of warning-severity violations.
    pub fn warning_count(&self) -> usize {
        self.violations
            .iter()
            .filter(|v| v.severity == Severity::Warning)
            .count()
    }
}

impl Default for ValidationReport {
    fn default() -> Self {
        Self::new()
    }
}
