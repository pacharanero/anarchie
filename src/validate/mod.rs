// SPDX-License-Identifier: AGPL-3.0-or-later
//! # anarchie-validate
//!
//! Validation is the price of admission for a CDR: storing files is easy,
//! storing only *valid* openEHR Compositions and rejecting malformed ones at
//! the door is what makes `anarchie` a CDR rather than a JSON folder.
//!
//! Validation has two layers:
//!
//! 1. **Reference Model** ([`validate_rm`]) - "is this openEHR at all?" Always
//!    on, needs no template.
//! 2. **Operational Template** ([`validate_opt`]) - "is this *this kind of*
//!    openEHR?" Walks the Composition against the flattened template it claims.
//!
//! Both produce a [`ValidationReport`] of addressable [`Violation`]s rather than
//! a bare boolean, so the commit path can reject on errors, the CLI can display
//! them, and the future MCP/LLM layer can feed them back for correction.
//!
//! This is a pure-Rust reimplementation: no JVM, no runtime dependency on
//! Archie or the openEHR SDK, preserving the single-binary promise. Archie is
//! used only as a *development-time* oracle in the cross-check harness.

mod opt;
mod report;
mod rm;

pub use crate::opt::{Opt, OptError};
pub use opt::validate_opt;
pub use report::{Severity, ValidationReport, Violation};
pub use rm::validate_rm;

use crate::rm::Composition;

/// Validate a Composition against the Reference Model and, if supplied, an
/// Operational Template. The RM layer always runs; the OPT layer runs only when
/// a template is given.
pub fn validate(composition: &Composition, opt: Option<&Opt>) -> ValidationReport {
    let mut report = validate_rm(composition);
    if let Some(opt) = opt {
        match serde_json::to_value(composition) {
            Ok(value) => report.merge(validate_opt(&value, opt)),
            Err(err) => report.error(
                "/",
                "OPT:serialise",
                format!("could not serialise composition for template validation: {err}"),
            ),
        }
    }
    report
}
