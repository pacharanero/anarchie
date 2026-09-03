// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lowering the openEHR SDK's AQL syntax tree into anarchie's query model.
//!
//! [`openehr_query`] parses the whole of AQL 1.1 against the official grammar.
//! anarchie executes a deliberate subset of it (see `specs/query-engine.md`),
//! so this module is the narrowing step: it accepts what the SQLite path index
//! can answer and rejects everything else by name.
//!
//! Rejecting here rather than at parse time is the point of the split. The
//! grammar decides what is *valid AQL*; this module decides what *anarchie can
//! execute*, and a query that is valid but unsupported gets a message saying
//! which construct is missing rather than a syntax error blaming the user.

use openehr_query::ast as sdk;
use openehr_query::lexer::CompOp;

use super::ast::{
    AggFunc, AqlQuery, CompareOp, Container, IdentifiedPath, OrderExpr, SelectExpr, SelectKind,
    Terminal, WhereExpr,
};

/// Parse AQL into anarchie's executable query model.
pub fn parse(input: &str) -> Result<AqlQuery, String> {
    let query = openehr_query::parser::parse_str(input).map_err(|error| error.to_string())?;
    lower(query)
}

fn unsupported(what: &str) -> String {
    format!("unsupported AQL: {what}")
}

fn lower(query: sdk::SelectQuery) -> Result<AqlQuery, String> {
    let (limit, offset) = match query.limit {
        Some(sdk::Limit { limit, offset }) => (Some(limit), offset),
        None => (None, None),
    };

    Ok(AqlQuery {
        distinct: query.select.distinct,
        top: query.select.top.map(lower_top).transpose()?,
        select: query
            .select
            .columns
            .into_iter()
            .map(lower_select)
            .collect::<Result<_, _>>()?,
        from: lower_from(query.from)?,
        where_clause: query.where_.map(lower_where).transpose()?,
        order_by: query
            .order_by
            .into_iter()
            .map(lower_order)
            .collect::<Result<_, _>>()?,
        limit,
        offset,
    })
}

/// `TOP n` without a direction. `TOP n FORWARD`/`BACKWARD` orders the window
/// relative to the version history, which the index does not model.
fn lower_top(top: sdk::Top) -> Result<i64, String> {
    match top.direction {
        None => Ok(top.count),
        Some(_) => Err(unsupported("TOP with a FORWARD or BACKWARD direction")),
    }
}

fn lower_select(expr: sdk::SelectExpr) -> Result<SelectExpr, String> {
    let kind = match expr.column {
        sdk::ColumnExpr::Path(path) => SelectKind::Path(lower_path(path)?),
        sdk::ColumnExpr::Aggregate(sdk::AggregateCall::Count { distinct: true, .. }) => {
            return Err(unsupported("COUNT(DISTINCT …)"))
        }
        sdk::ColumnExpr::Aggregate(sdk::AggregateCall::Count { path: None, .. }) => {
            SelectKind::CountStar
        }
        sdk::ColumnExpr::Aggregate(sdk::AggregateCall::Count {
            path: Some(path), ..
        }) => SelectKind::Aggregate {
            func: AggFunc::Count,
            arg: lower_path(path)?,
        },
        sdk::ColumnExpr::Aggregate(sdk::AggregateCall::Stat { func, path }) => {
            SelectKind::Aggregate {
                func: match func {
                    sdk::StatFunc::Min => AggFunc::Min,
                    sdk::StatFunc::Max => AggFunc::Max,
                    sdk::StatFunc::Sum => AggFunc::Sum,
                    sdk::StatFunc::Avg => AggFunc::Avg,
                },
                arg: lower_path(path)?,
            }
        }
        sdk::ColumnExpr::Primitive(_) => return Err(unsupported("a literal SELECT column")),
        sdk::ColumnExpr::Function(_) => return Err(unsupported("function calls in SELECT")),
    };
    Ok(SelectExpr {
        kind,
        alias: expr.alias,
    })
}

/// Flatten the `FROM … CONTAINS …` spine.
///
/// anarchie models containment as a linear chain, so a branching or negated
/// `CONTAINS` is rejected rather than silently losing a branch.
fn lower_from(expr: sdk::ContainsExpr) -> Result<Vec<Container>, String> {
    let mut chain = Vec::new();
    let mut next = Some(expr);

    while let Some(current) = next.take() {
        let (operand, contains) = match current {
            sdk::ContainsExpr::Contained { operand, contains } => (operand, contains),
            sdk::ContainsExpr::And(..) | sdk::ContainsExpr::Or(..) => {
                return Err(unsupported("AND/OR between CONTAINS operands"))
            }
        };

        chain.push(lower_class(operand)?);

        if let Some(constraint) = contains {
            if constraint.negated {
                return Err(unsupported("NOT CONTAINS"));
            }
            next = Some(constraint.expr);
        }
    }

    Ok(chain)
}

fn lower_class(operand: sdk::ClassExprOperand) -> Result<Container, String> {
    let sdk::ClassExprOperand::Class {
        rm_type,
        variable,
        predicate,
    } = operand
    else {
        return Err(unsupported("the VERSION class expression"));
    };

    Ok(Container {
        rm_type,
        variable,
        archetype_id: predicate.map(archetype_id).transpose()?,
    })
}

/// The archetype HRID constraining a `FROM` operand, e.g. the
/// `openEHR-EHR-OBSERVATION.blood_pressure.v2` in `OBSERVATION o[…]`.
fn archetype_id(predicate: sdk::PathPredicate) -> Result<String, String> {
    match predicate {
        sdk::PathPredicate::Archetype(sdk::ArchetypePredicate::Hrid(hrid)) => Ok(hrid),
        sdk::PathPredicate::Node(node) => match *node {
            sdk::NodePredicate::Archetype { hrid, .. } => Ok(hrid),
            _ => Err(unsupported(
                "this class predicate; expected an archetype id",
            )),
        },
        _ => Err(unsupported(
            "this class predicate; expected an archetype id",
        )),
    }
}

fn lower_path(path: sdk::IdentifiedPath) -> Result<IdentifiedPath, String> {
    if path.predicate.is_some() {
        return Err(unsupported("a predicate on the root of a data path"));
    }
    Ok(IdentifiedPath {
        variable: path.root,
        path: path.path.map(render_object_path).transpose()?,
    })
}

/// Render an object path in the form the index stores.
///
/// Only the node code is emitted from a predicate. A trailing name or term
/// constraint (`items[at0004, 'Systolic']`) narrows by display name, which the
/// index does not record, so it is dropped from the path text exactly as the
/// previous hand-written parser did. Predicates that would *change* which node
/// is selected are rejected instead, because dropping those would silently
/// widen the query.
fn render_object_path(path: sdk::ObjectPath) -> Result<String, String> {
    let mut out = String::new();
    for (index, part) in path.parts.iter().enumerate() {
        if index > 0 {
            out.push('/');
        }
        out.push_str(&part.name);
        if let Some(predicate) = &part.predicate {
            out.push('[');
            out.push_str(&node_code(predicate)?);
            out.push(']');
        }
    }
    Ok(out)
}

fn node_code(predicate: &sdk::PathPredicate) -> Result<String, String> {
    match predicate {
        sdk::PathPredicate::Archetype(sdk::ArchetypePredicate::Hrid(hrid)) => Ok(hrid.clone()),
        sdk::PathPredicate::Node(node) => match node.as_ref() {
            sdk::NodePredicate::Code { code, .. } => Ok(code.clone()),
            sdk::NodePredicate::Archetype { hrid, .. } => Ok(hrid.clone()),
            _ => Err(unsupported("this path predicate; expected a node id")),
        },
        sdk::PathPredicate::Standard(_) => Err(unsupported("a comparison predicate in a path")),
        sdk::PathPredicate::Archetype(sdk::ArchetypePredicate::Parameter(_)) => {
            Err(unsupported("a parameter as a node id"))
        }
    }
}

fn lower_where(expr: sdk::WhereExpr) -> Result<WhereExpr, String> {
    Ok(match expr {
        sdk::WhereExpr::Not(inner) => WhereExpr::Not(Box::new(lower_where(*inner)?)),
        sdk::WhereExpr::And(left, right) => WhereExpr::And(
            Box::new(lower_where(*left)?),
            Box::new(lower_where(*right)?),
        ),
        sdk::WhereExpr::Or(left, right) => WhereExpr::Or(
            Box::new(lower_where(*left)?),
            Box::new(lower_where(*right)?),
        ),
        sdk::WhereExpr::Identified(leaf) => lower_condition(leaf)?,
    })
}

fn lower_condition(expr: sdk::IdentifiedExpr) -> Result<WhereExpr, String> {
    Ok(match expr {
        sdk::IdentifiedExpr::Exists(path) => WhereExpr::Exists(lower_path(path)?),
        sdk::IdentifiedExpr::Compare { lhs, op, rhs } => {
            let sdk::CompareOperand::Path(path) = lhs else {
                return Err(unsupported("function calls in WHERE"));
            };
            WhereExpr::Compare {
                path: lower_path(path)?,
                op: lower_op(op),
                value: lower_terminal(rhs)?,
            }
        }
        sdk::IdentifiedExpr::Like { path, operand } => {
            let sdk::LikeOperand::String(pattern) = operand else {
                return Err(unsupported("a parameter as a LIKE pattern"));
            };
            WhereExpr::Like {
                path: lower_path(path)?,
                pattern,
            }
        }
        sdk::IdentifiedExpr::Matches { path, operand } => {
            let sdk::MatchesOperand::ValueList(items) = operand else {
                return Err(unsupported("MATCHES against a terminology set or URI"));
            };
            WhereExpr::Matches {
                path: lower_path(path)?,
                values: items
                    .into_iter()
                    .map(lower_value_list_item)
                    .collect::<Result<_, _>>()?,
            }
        }
        // Only semantic analysis produces this, and anarchie does not run it.
        sdk::IdentifiedExpr::Resolved(_) => return Err(unsupported("a resolved constant")),
    })
}

fn lower_op(op: CompOp) -> CompareOp {
    match op {
        CompOp::Eq => CompareOp::Eq,
        CompOp::Ne => CompareOp::Ne,
        CompOp::Lt => CompareOp::Lt,
        CompOp::Le => CompareOp::Le,
        CompOp::Gt => CompareOp::Gt,
        CompOp::Ge => CompareOp::Ge,
    }
}

fn lower_terminal(terminal: sdk::Terminal) -> Result<Terminal, String> {
    match terminal {
        sdk::Terminal::Primitive(primitive) => lower_primitive(primitive),
        sdk::Terminal::Parameter(name) => Ok(Terminal::Param(strip_parameter(&name))),
        sdk::Terminal::Path(_) => Err(unsupported("comparing two data paths")),
        sdk::Terminal::Function(_) => Err(unsupported("function calls in WHERE")),
    }
}

fn lower_value_list_item(item: sdk::ValueListItem) -> Result<Terminal, String> {
    match item {
        sdk::ValueListItem::Primitive(primitive) => lower_primitive(primitive),
        sdk::ValueListItem::Parameter(name) => Ok(Terminal::Param(strip_parameter(&name))),
        sdk::ValueListItem::Terminology(_) => {
            Err(unsupported("a terminology function inside MATCHES"))
        }
    }
}

fn lower_primitive(primitive: sdk::Primitive) -> Result<Terminal, String> {
    match primitive {
        sdk::Primitive::String(value) => Ok(Terminal::String(value)),
        sdk::Primitive::Integer(value) => Ok(Terminal::Number(value as f64)),
        sdk::Primitive::Real(value) => Ok(Terminal::Number(value)),
        sdk::Primitive::Boolean(value) => Ok(Terminal::Bool(value)),
        sdk::Primitive::Null => Err(unsupported("the NULL literal")),
    }
}

/// Parameter names are bound without their `$` sigil.
fn strip_parameter(name: &str) -> String {
    name.strip_prefix('$').unwrap_or(name).to_string()
}

fn lower_order(order: sdk::OrderByExpr) -> Result<OrderExpr, String> {
    Ok(OrderExpr {
        path: lower_path(order.path)?,
        descending: matches!(order.order, Some(sdk::SortOrder::Descending)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_full_mvp_query() {
        let aql = "SELECT o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude AS systolic \
                   FROM EHR e CONTAINS COMPOSITION c CONTAINS OBSERVATION o[openEHR-EHR-OBSERVATION.blood_pressure.v2] \
                   WHERE o/data[at0001]/events[at0006]/data[at0003]/items[at0004]/value/magnitude > 140 \
                   ORDER BY systolic DESC LIMIT 10 OFFSET 5";
        let q = parse(aql).expect("parses");
        assert_eq!(q.select.len(), 1);
        assert_eq!(q.select[0].alias.as_deref(), Some("systolic"));
        assert_eq!(q.from.len(), 3);
        assert_eq!(q.from[2].rm_type, "OBSERVATION");
        assert_eq!(
            q.from[2].archetype_id.as_deref(),
            Some("openEHR-EHR-OBSERVATION.blood_pressure.v2")
        );
        assert!(q.where_clause.is_some());
        assert_eq!(q.limit, Some(10));
        assert_eq!(q.offset, Some(5));
        assert_eq!(q.order_by.len(), 1);
        assert!(q.order_by[0].descending);
    }

    #[test]
    fn parses_count_star_and_params() {
        let q = parse(
            "SELECT COUNT(*) FROM EHR e CONTAINS COMPOSITION c WHERE e/ehr_id/value = $ehrUid",
        )
        .expect("parses");
        assert!(matches!(q.select[0].kind, SelectKind::CountStar));
        match q.where_clause.unwrap() {
            WhereExpr::Compare { value, .. } => assert_eq!(value, Terminal::Param("ehrUid".into())),
            other => panic!("unexpected where: {other:?}"),
        }
    }

    #[test]
    fn parses_matches_and_boolean_logic() {
        let q = parse(
            "SELECT c/name/value FROM COMPOSITION c \
             WHERE c/name/value MATCHES {'a', 'b'} AND NOT c/name/value LIKE 'x%'",
        )
        .expect("parses");
        match q.where_clause.unwrap() {
            WhereExpr::And(l, r) => {
                assert!(matches!(*l, WhereExpr::Matches { .. }));
                assert!(matches!(*r, WhereExpr::Not(_)));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    /// A name constraint narrows by display name, which the index does not
    /// record. Dropping it must not change the path the index is asked for.
    #[test]
    fn a_node_name_constraint_does_not_reach_the_index_path() {
        let with_name = parse("SELECT o/data[at0001, 'Systolic']/value FROM OBSERVATION o")
            .expect("a named node predicate parses");
        let without = parse("SELECT o/data[at0001]/value FROM OBSERVATION o").expect("parses");

        assert_eq!(with_name.select, without.select);
        let SelectKind::Path(path) = &with_name.select[0].kind else {
            panic!("expected a path column");
        };
        assert_eq!(path.path.as_deref(), Some("data[at0001]/value"));
    }

    /// Valid AQL that anarchie cannot execute must be refused by name, not
    /// silently narrowed to something it can run.
    #[test]
    fn valid_but_unsupported_aql_is_refused_by_name() {
        for (aql, expected) in [
            (
                "SELECT c/name/value FROM EHR e CONTAINS (COMPOSITION c OR FOLDER f)",
                "AND/OR between CONTAINS operands",
            ),
            (
                "SELECT c/name/value FROM EHR e NOT CONTAINS COMPOSITION c",
                "NOT CONTAINS",
            ),
            (
                "SELECT LENGTH(c/name/value) FROM COMPOSITION c",
                "function calls in SELECT",
            ),
            (
                "SELECT COUNT(DISTINCT c/name/value) FROM COMPOSITION c",
                "COUNT(DISTINCT …)",
            ),
            (
                "SELECT v/data FROM EHR e CONTAINS VERSION v",
                "the VERSION class expression",
            ),
            (
                "SELECT c/name/value FROM COMPOSITION c WHERE c/name/value = d/name/value",
                "comparing two data paths",
            ),
        ] {
            let error = parse(aql).expect_err(&format!("{aql} must be refused"));
            assert!(
                error.contains(expected),
                "{aql}\n  expected a refusal naming {expected:?}, got {error:?}"
            );
        }
    }
}
