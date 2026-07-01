// SPDX-FileCopyrightText: 2026 Marcus Baw and Baw Medical Ltd
// SPDX-License-Identifier: AGPL-3.0-or-later
//! A small hand-written tokeniser for the AQL MVP subset.
//!
//! Identifiers are lexed greedily to include `.` and `-` so that archetype ids
//! (`openEHR-EHR-OBSERVATION.blood_pressure.v2`) and at-codes (`at0006`) are a
//! single token; keyword recognition is left to the parser (case-insensitive).

/// A lexical token.
#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    /// An identifier, archetype id, at-code, RM type, or (unresolved) keyword.
    Ident(String),
    Number(f64),
    /// A single-quoted string literal (quotes stripped).
    Str(String),
    Slash,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Dollar,
    Star,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Tokenise `input`, or return a human-readable error string.
pub fn lex(input: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        match c {
            c if c.is_whitespace() => i += 1,
            '/' => {
                tokens.push(Token::Slash);
                i += 1;
            }
            '[' => {
                tokens.push(Token::LBracket);
                i += 1;
            }
            ']' => {
                tokens.push(Token::RBracket);
                i += 1;
            }
            '{' => {
                tokens.push(Token::LBrace);
                i += 1;
            }
            '}' => {
                tokens.push(Token::RBrace);
                i += 1;
            }
            '(' => {
                tokens.push(Token::LParen);
                i += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                i += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                i += 1;
            }
            '$' => {
                tokens.push(Token::Dollar);
                i += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                i += 1;
            }
            '=' => {
                tokens.push(Token::Eq);
                i += 1;
            }
            '!' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push(Token::Ne);
                    i += 2;
                } else {
                    return Err("unexpected '!' (did you mean '!='?)".into());
                }
            }
            '<' => match chars.get(i + 1) {
                Some('=') => {
                    tokens.push(Token::Le);
                    i += 2;
                }
                Some('>') => {
                    tokens.push(Token::Ne);
                    i += 2;
                }
                _ => {
                    tokens.push(Token::Lt);
                    i += 1;
                }
            },
            '>' => {
                if chars.get(i + 1) == Some(&'=') {
                    tokens.push(Token::Ge);
                    i += 2;
                } else {
                    tokens.push(Token::Gt);
                    i += 1;
                }
            }
            '\'' => {
                let (s, next) = lex_string(&chars, i)?;
                tokens.push(Token::Str(s));
                i = next;
            }
            c if c.is_ascii_digit() || (c == '-' && next_is_digit(&chars, i)) => {
                let (n, next) = lex_number(&chars, i)?;
                tokens.push(Token::Number(n));
                i = next;
            }
            c if is_ident_start(c) => {
                let (s, next) = lex_ident(&chars, i);
                tokens.push(Token::Ident(s));
                i = next;
            }
            other => return Err(format!("unexpected character {other:?}")),
        }
    }
    Ok(tokens)
}

fn next_is_digit(chars: &[char], i: usize) -> bool {
    chars.get(i + 1).is_some_and(|c| c.is_ascii_digit())
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Identifier body: letters, digits, `_`, `.`, `-` (so archetype ids lex whole).
fn is_ident_body(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-'
}

fn lex_ident(chars: &[char], start: usize) -> (String, usize) {
    let mut i = start + 1;
    while i < chars.len() && is_ident_body(chars[i]) {
        i += 1;
    }
    (chars[start..i].iter().collect(), i)
}

fn lex_number(chars: &[char], start: usize) -> Result<(f64, usize), String> {
    let mut i = start;
    if chars[i] == '-' {
        i += 1;
    }
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        i += 1;
    }
    let s: String = chars[start..i].iter().collect();
    s.parse::<f64>()
        .map(|n| (n, i))
        .map_err(|_| format!("invalid number {s:?}"))
}

fn lex_string(chars: &[char], start: usize) -> Result<(String, usize), String> {
    let mut i = start + 1;
    let mut s = String::new();
    while i < chars.len() {
        match chars[i] {
            '\'' => return Ok((s, i + 1)),
            '\\' if i + 1 < chars.len() => {
                s.push(chars[i + 1]);
                i += 2;
            }
            c => {
                s.push(c);
                i += 1;
            }
        }
    }
    Err("unterminated string literal".into())
}
