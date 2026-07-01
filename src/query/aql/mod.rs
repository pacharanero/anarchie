// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! AQL (Archetype Query Language) for the MVP subset: lexer, AST and parser.

mod ast;
mod lexer;
mod parser;

pub use ast::*;
pub use parser::parse;
