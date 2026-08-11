// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clinical knowledge inventory and package management.
//!
//! The first implemented slice is a deterministic, read-only inventory of an
//! openEHR CKM mirror checkout. Resolution, locking, installation, and package
//! activation build on this evidence model; see `specs/knowledge-packages.md`.

mod inventory;

pub use inventory::{
    Artifact, ArtifactKind, DependencyIssue, DependencyIssueKind, Inventory, InventoryError,
    InventorySummary, SourceEvidence,
};
