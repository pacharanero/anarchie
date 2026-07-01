// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! A recursive-descent parser for the AQL MVP subset, over [`Token`]s.

use super::ast::*;
use super::lexer::{lex, Token};

/// Parse AQL text into an [`AqlQuery`], or return a human-readable error.
pub fn parse(input: &str) -> Result<AqlQuery, String> {
    let tokens = lex(input)?;
    let mut parser = Parser { tokens, pos: 0 };
    let query = parser.parse_query()?;
    if parser.pos != parser.tokens.len() {
        return Err(format!(
            "unexpected trailing input near token {}",
            parser.pos + 1
        ));
    }
    Ok(query)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    /// The uppercased identifier at the cursor, if any (for keyword checks).
    fn peek_ident_upper(&self) -> Option<String> {
        match self.peek() {
            Some(Token::Ident(s)) => Some(s.to_ascii_uppercase()),
            _ => None,
        }
    }

    fn is_kw(&self, kw: &str) -> bool {
        self.peek_ident_upper().as_deref() == Some(kw)
    }

    /// Consume an identifier token if it equals `kw` (case-insensitive).
    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.is_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, want: &Token) -> Result<(), String> {
        match self.next() {
            Some(ref t) if t == want => Ok(()),
            other => Err(format!("expected {want:?}, found {other:?}")),
        }
    }

    fn expect_ident(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token::Ident(s)) => Ok(s),
            other => Err(format!("expected identifier, found {other:?}")),
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<(), String> {
        if self.eat_kw(kw) {
            Ok(())
        } else {
            Err(format!("expected keyword {kw}, found {:?}", self.peek()))
        }
    }

    fn parse_query(&mut self) -> Result<AqlQuery, String> {
        self.expect_kw("SELECT")?;
        let distinct = self.eat_kw("DISTINCT");
        let top = if self.eat_kw("TOP") {
            Some(self.expect_integer()?)
        } else {
            None
        };

        let select = self.parse_select_list()?;
        self.expect_kw("FROM")?;
        let from = self.parse_from()?;

        let where_clause = if self.eat_kw("WHERE") {
            Some(self.parse_or()?)
        } else {
            None
        };

        let mut order_by = Vec::new();
        if self.eat_kw("ORDER") {
            self.expect_kw("BY")?;
            order_by = self.parse_order_list()?;
        }

        let mut limit = None;
        let mut offset = None;
        if self.eat_kw("LIMIT") {
            limit = Some(self.expect_integer()?);
            if self.eat_kw("OFFSET") {
                offset = Some(self.expect_integer()?);
            }
        }

        Ok(AqlQuery {
            distinct,
            top,
            select,
            from,
            where_clause,
            order_by,
            limit,
            offset,
        })
    }

    fn parse_select_list(&mut self) -> Result<Vec<SelectExpr>, String> {
        let mut exprs = vec![self.parse_select_expr()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.pos += 1;
            exprs.push(self.parse_select_expr()?);
        }
        Ok(exprs)
    }

    fn parse_select_expr(&mut self) -> Result<SelectExpr, String> {
        let kind = if let Some(func) = self.peek_agg_func() {
            // Aggregate only when followed by '(' — otherwise it's a variable.
            if matches!(self.tokens.get(self.pos + 1), Some(Token::LParen)) {
                self.pos += 1; // function name
                self.expect(&Token::LParen)?;
                let kind = if matches!(self.peek(), Some(Token::Star)) {
                    self.pos += 1;
                    if func != AggFunc::Count {
                        return Err("only COUNT(*) is supported".into());
                    }
                    SelectKind::CountStar
                } else {
                    SelectKind::Aggregate {
                        func,
                        arg: self.parse_identified_path()?,
                    }
                };
                self.expect(&Token::RParen)?;
                kind
            } else {
                SelectKind::Path(self.parse_identified_path()?)
            }
        } else {
            SelectKind::Path(self.parse_identified_path()?)
        };

        let alias = if self.eat_kw("AS") {
            Some(self.expect_ident()?)
        } else {
            None
        };
        Ok(SelectExpr { kind, alias })
    }

    fn peek_agg_func(&self) -> Option<AggFunc> {
        match self.peek_ident_upper().as_deref() {
            Some("COUNT") => Some(AggFunc::Count),
            Some("MIN") => Some(AggFunc::Min),
            Some("MAX") => Some(AggFunc::Max),
            Some("SUM") => Some(AggFunc::Sum),
            Some("AVG") => Some(AggFunc::Avg),
            _ => None,
        }
    }

    /// `variable ( '/' object_path )?`
    fn parse_identified_path(&mut self) -> Result<IdentifiedPath, String> {
        let variable = self.expect_ident()?;
        let path = if matches!(self.peek(), Some(Token::Slash)) {
            self.pos += 1;
            Some(self.parse_object_path()?)
        } else {
            None
        };
        Ok(IdentifiedPath { variable, path })
    }

    /// `segment ( '/' segment )*`, each `ident ( '[' predicate ']' )?`.
    fn parse_object_path(&mut self) -> Result<String, String> {
        let mut out = String::new();
        loop {
            let attr = self.expect_ident()?;
            out.push_str(&attr);
            if matches!(self.peek(), Some(Token::LBracket)) {
                self.pos += 1;
                let node_id = self.expect_ident()?;
                out.push('[');
                out.push_str(&node_id);
                out.push(']');
                // Optional `, name` predicate is accepted but ignored for path
                // matching in the MVP.
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.pos += 1;
                    match self.next() {
                        Some(Token::Str(_)) | Some(Token::Ident(_)) => {}
                        other => return Err(format!("expected name predicate, found {other:?}")),
                    }
                }
                self.expect(&Token::RBracket)?;
            }
            if matches!(self.peek(), Some(Token::Slash)) {
                self.pos += 1;
                out.push('/');
            } else {
                break;
            }
        }
        Ok(out)
    }

    /// `class_expr ( CONTAINS class_expr )*`
    fn parse_from(&mut self) -> Result<Vec<Container>, String> {
        let mut chain = vec![self.parse_container()?];
        while self.eat_kw("CONTAINS") {
            chain.push(self.parse_container()?);
        }
        Ok(chain)
    }

    /// `RM_TYPE variable? ( '[' archetype_id ']' )?`
    fn parse_container(&mut self) -> Result<Container, String> {
        let rm_type = self.expect_ident()?.to_ascii_uppercase();
        let variable = if matches!(self.peek(), Some(Token::Ident(_)))
            && !matches!(self.peek(), Some(Token::LBracket))
            && !self.is_kw("CONTAINS")
            && !self.is_kw("WHERE")
            && !self.is_kw("ORDER")
            && !self.is_kw("LIMIT")
        {
            Some(self.expect_ident()?)
        } else {
            None
        };
        let archetype_id = if matches!(self.peek(), Some(Token::LBracket)) {
            self.pos += 1;
            let id = self.expect_ident()?;
            self.expect(&Token::RBracket)?;
            Some(id)
        } else {
            None
        };
        Ok(Container {
            rm_type,
            variable,
            archetype_id,
        })
    }

    // WHERE: OR is lowest precedence, then AND, then NOT, then primary.
    fn parse_or(&mut self) -> Result<WhereExpr, String> {
        let mut left = self.parse_and()?;
        while self.eat_kw("OR") {
            let right = self.parse_and()?;
            left = WhereExpr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<WhereExpr, String> {
        let mut left = self.parse_not()?;
        while self.eat_kw("AND") {
            let right = self.parse_not()?;
            left = WhereExpr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_not(&mut self) -> Result<WhereExpr, String> {
        if self.eat_kw("NOT") {
            Ok(WhereExpr::Not(Box::new(self.parse_not()?)))
        } else {
            self.parse_where_primary()
        }
    }

    fn parse_where_primary(&mut self) -> Result<WhereExpr, String> {
        if matches!(self.peek(), Some(Token::LParen)) {
            self.pos += 1;
            let inner = self.parse_or()?;
            self.expect(&Token::RParen)?;
            return Ok(inner);
        }
        if self.eat_kw("EXISTS") {
            return Ok(WhereExpr::Exists(self.parse_identified_path()?));
        }

        let path = self.parse_identified_path()?;
        if self.eat_kw("MATCHES") {
            self.expect(&Token::LBrace)?;
            let mut values = vec![self.parse_terminal()?];
            while matches!(self.peek(), Some(Token::Comma)) {
                self.pos += 1;
                values.push(self.parse_terminal()?);
            }
            self.expect(&Token::RBrace)?;
            return Ok(WhereExpr::Matches { path, values });
        }
        if self.eat_kw("LIKE") {
            let pattern = match self.next() {
                Some(Token::Str(s)) => s,
                other => return Err(format!("expected string after LIKE, found {other:?}")),
            };
            return Ok(WhereExpr::Like { path, pattern });
        }
        let op = self.parse_compare_op()?;
        let value = self.parse_terminal()?;
        Ok(WhereExpr::Compare { path, op, value })
    }

    fn parse_compare_op(&mut self) -> Result<CompareOp, String> {
        let op = match self.next() {
            Some(Token::Eq) => CompareOp::Eq,
            Some(Token::Ne) => CompareOp::Ne,
            Some(Token::Lt) => CompareOp::Lt,
            Some(Token::Le) => CompareOp::Le,
            Some(Token::Gt) => CompareOp::Gt,
            Some(Token::Ge) => CompareOp::Ge,
            other => return Err(format!("expected comparison operator, found {other:?}")),
        };
        Ok(op)
    }

    fn parse_terminal(&mut self) -> Result<Terminal, String> {
        match self.peek() {
            Some(Token::Dollar) => {
                self.pos += 1;
                Ok(Terminal::Param(self.expect_ident()?))
            }
            Some(Token::Number(n)) => {
                let n = *n;
                self.pos += 1;
                Ok(Terminal::Number(n))
            }
            Some(Token::Str(s)) => {
                let s = s.clone();
                self.pos += 1;
                Ok(Terminal::String(s))
            }
            Some(Token::Ident(s)) => {
                let s = s.clone();
                self.pos += 1;
                match s.to_ascii_uppercase().as_str() {
                    "TRUE" => Ok(Terminal::Bool(true)),
                    "FALSE" => Ok(Terminal::Bool(false)),
                    _ => Err(format!("unexpected identifier {s:?} as a value")),
                }
            }
            other => Err(format!("expected a value, found {other:?}")),
        }
    }

    fn parse_order_list(&mut self) -> Result<Vec<OrderExpr>, String> {
        let mut out = vec![self.parse_order_expr()?];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.pos += 1;
            out.push(self.parse_order_expr()?);
        }
        Ok(out)
    }

    fn parse_order_expr(&mut self) -> Result<OrderExpr, String> {
        let path = self.parse_identified_path()?;
        let descending = if self.eat_kw("DESC") {
            true
        } else {
            self.eat_kw("ASC");
            false
        };
        Ok(OrderExpr { path, descending })
    }

    fn expect_integer(&mut self) -> Result<i64, String> {
        match self.next() {
            Some(Token::Number(n)) if n.fract() == 0.0 => Ok(n as i64),
            other => Err(format!("expected an integer, found {other:?}")),
        }
    }
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
}
