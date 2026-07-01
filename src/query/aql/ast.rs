// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! The AQL abstract syntax tree for the MVP subset.
//!
//! Covers `SELECT` of identified paths and aggregates, a linear
//! `FROM … CONTAINS …` chain, a `WHERE` of comparisons / `MATCHES` / `EXISTS`
//! combined with `AND`/`OR`/`NOT`, and `ORDER BY` / `LIMIT` / `OFFSET`. See
//! `specs/query-engine.md` for the deferred surface.

/// A parsed AQL query.
#[derive(Clone, Debug, PartialEq)]
pub struct AqlQuery {
    pub distinct: bool,
    pub top: Option<i64>,
    pub select: Vec<SelectExpr>,
    pub from: Vec<Container>,
    pub where_clause: Option<WhereExpr>,
    pub order_by: Vec<OrderExpr>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// One `SELECT` column: a leaf path or an aggregate over one.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectExpr {
    pub kind: SelectKind,
    pub alias: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SelectKind {
    /// A leaf identified path, e.g. `o/.../value/magnitude`.
    Path(IdentifiedPath),
    /// An aggregate over an identified path, e.g. `AVG(o/.../magnitude)`.
    Aggregate { func: AggFunc, arg: IdentifiedPath },
    /// `COUNT(*)`.
    CountStar,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AggFunc {
    Count,
    Min,
    Max,
    Sum,
    Avg,
}

impl AggFunc {
    pub fn sql(self) -> &'static str {
        match self {
            AggFunc::Count => "COUNT",
            AggFunc::Min => "MIN",
            AggFunc::Max => "MAX",
            AggFunc::Sum => "SUM",
            AggFunc::Avg => "AVG",
        }
    }
}

/// A path rooted at a `FROM` variable: `variable` then an optional object path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentifiedPath {
    pub variable: String,
    /// The canonical path after the variable, e.g.
    /// `data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude`,
    /// or `None` when the path is just the variable itself.
    pub path: Option<String>,
}

/// One element of the `FROM … CONTAINS …` chain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Container {
    pub rm_type: String,
    pub variable: Option<String>,
    pub archetype_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WhereExpr {
    Compare {
        path: IdentifiedPath,
        op: CompareOp,
        value: Terminal,
    },
    Matches {
        path: IdentifiedPath,
        values: Vec<Terminal>,
    },
    Like {
        path: IdentifiedPath,
        pattern: String,
    },
    Exists(IdentifiedPath),
    Not(Box<WhereExpr>),
    And(Box<WhereExpr>, Box<WhereExpr>),
    Or(Box<WhereExpr>, Box<WhereExpr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl CompareOp {
    pub fn sql(self) -> &'static str {
        match self {
            CompareOp::Eq => "=",
            CompareOp::Ne => "<>",
            CompareOp::Lt => "<",
            CompareOp::Le => "<=",
            CompareOp::Gt => ">",
            CompareOp::Ge => ">=",
        }
    }
}

/// A literal or parameter on the right of a comparison / inside `MATCHES`.
#[derive(Clone, Debug, PartialEq)]
pub enum Terminal {
    Number(f64),
    String(String),
    Bool(bool),
    /// A `$name` parameter, resolved at execution time.
    Param(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OrderExpr {
    pub path: IdentifiedPath,
    pub descending: bool,
}
