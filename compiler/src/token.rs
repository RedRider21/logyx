// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Token del linguaggio Logyx (porting in Rust del lexer del prototipo Python).

/// Pezzo di una stringa: testo letterale oppure il sorgente di un'interpolazione `{...}`.
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    Lit(String),
    Expr(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // letterali
    Int(i64),
    Float(f64),
    Str(Vec<StringPart>),
    True,
    False,
    Nil,
    Ident(String),
    /// Template HTML grezzo catturato dopo `render` (l'interpolazione si risolve in codegen).
    Template(String),
    // parole chiave
    Fn,
    Return,
    If,
    Else,
    While,
    For,
    In,
    Const,
    Import,
    Fail,
    Match,
    Break,
    Continue,
    Record,
    Enum,
    Use,
    Extern,
    Render,
    And,
    Or,
    Not,
    // punteggiatura e operatori
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBrack,
    RBrack,
    Comma,
    Colon,
    Arrow,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    PercentEq,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Question,
    Pipe,
    Dot,
    Eof,
}

/// Token con la posizione nel sorgente (per i messaggi d'errore).
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

/// Restituisce la parola chiave corrispondente a un identificatore, se esiste.
pub fn keyword(word: &str) -> Option<TokenKind> {
    let k = match word {
        "fn" => TokenKind::Fn,
        "return" => TokenKind::Return,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "for" => TokenKind::For,
        "in" => TokenKind::In,
        "const" => TokenKind::Const,
        "import" => TokenKind::Import,
        "fail" => TokenKind::Fail,
        "match" => TokenKind::Match,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "record" => TokenKind::Record,
        "enum" => TokenKind::Enum,
        "use" => TokenKind::Use,
        "extern" => TokenKind::Extern,
        "and" => TokenKind::And,
        "or" => TokenKind::Or,
        "not" => TokenKind::Not,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "nil" => TokenKind::Nil,
        _ => return None,
    };
    Some(k)
}
