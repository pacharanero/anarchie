// SPDX-License-Identifier: AGPL-3.0-or-later
//! `anarchie`: a local-first, git-native, flat-file openEHR clinical data
//! repository (CDR).
//!
//! The library is organised into layered modules mirroring the openEHR stack:
//! [`rm`] (Reference Model), [`aom`] (Archetype Object Model constraints),
//! [`opt`] (Operational Template parsing), [`validate`] (RM + Operational
//! Template validation), [`store`] (the git-backed file store), [`query`] (the
//! AQL engine over a derived index), and [`serve`] (the openEHR REST API and
//! the MCP server). The `anarchie` binary is a thin CLI over these.

pub mod aom;
pub mod opt;
pub mod query;
pub mod rm;
pub mod serve;
pub mod store;
pub mod validate;
