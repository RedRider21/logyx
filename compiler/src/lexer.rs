// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lexer: dal sorgente Logyx alla lista di token. Porting del lexer del prototipo.
//!
//! Oltre al nucleo, riconosce il lato **server** del web (Fase 2a): `route` e
//! `render`, con cattura grezza del template HTML (token `Template`). Le isole
//! client `@start-client … @end-client` restano dentro il template (gestite più
//! avanti); un `@` fuori da un template è un errore.

use crate::error::LogyxError;
use crate::token::{keyword, StringPart, Token, TokenKind};

pub struct Lexer {
    src: Vec<char>,
    i: usize,
    line: usize,
    col: usize,
    file: String,
    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(src: &str, file: &str) -> Self {
        Lexer {
            src: src.chars().collect(),
            i: 0,
            line: 1,
            col: 1,
            file: file.to_string(),
            tokens: Vec::new(),
        }
    }

    fn err(&self, msg: &str) -> LogyxError {
        LogyxError::new(format!(
            "{}:{}:{}: errore lessicale: {}",
            self.file, self.line, self.col, msg
        ))
    }

    fn peek(&self, k: usize) -> char {
        self.src.get(self.i + k).copied().unwrap_or('\0')
    }

    fn advance(&mut self) -> char {
        let c = self.src[self.i];
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        c
    }

    fn push(&mut self, kind: TokenKind, line: usize, col: usize) {
        self.tokens.push(Token { kind, line, col });
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LogyxError> {
        while self.i < self.src.len() {
            let c = self.peek(0);
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                self.advance();
                continue;
            }
            if c == '/' && self.peek(1) == '/' {
                while self.i < self.src.len() && self.peek(0) != '\n' {
                    self.advance();
                }
                continue;
            }
            if c == '/' && self.peek(1) == '*' {
                self.advance();
                self.advance();
                while self.i < self.src.len() && !(self.peek(0) == '*' && self.peek(1) == '/') {
                    self.advance();
                }
                if self.i < self.src.len() {
                    self.advance();
                    self.advance();
                }
                continue;
            }
            if c == '@' {
                return Err(self.err(
                    "'@' inatteso: le isole client (@start-client … @end-client) sono \
                     ammesse solo dentro un template dopo 'render'",
                ));
            }
            if c == '"' {
                self.string()?;
                continue;
            }
            if c.is_ascii_digit() {
                self.number();
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                self.ident()?;
                continue;
            }
            self.operator()?;
        }
        self.push(TokenKind::Eof, self.line, self.col);
        Ok(self.tokens)
    }

    fn number(&mut self) {
        let (sl, sc) = (self.line, self.col);
        let start = self.i;
        while self.peek(0).is_ascii_digit() {
            self.advance();
        }
        let mut is_float = false;
        if self.peek(0) == '.' && self.peek(1).is_ascii_digit() {
            is_float = true;
            self.advance();
            while self.peek(0).is_ascii_digit() {
                self.advance();
            }
        }
        let text: String = self.src[start..self.i].iter().collect();
        if is_float {
            self.push(TokenKind::Float(text.parse().unwrap()), sl, sc);
        } else {
            self.push(TokenKind::Int(text.parse().unwrap()), sl, sc);
        }
    }

    fn ident(&mut self) -> Result<(), LogyxError> {
        let (sl, sc) = (self.line, self.col);
        let start = self.i;
        while self.peek(0).is_alphanumeric() || self.peek(0) == '_' {
            self.advance();
        }
        let text: String = self.src[start..self.i].iter().collect();
        if text == "render" {
            // `render` innesca la cattura grezza del template HTML che segue.
            self.push(TokenKind::Render, sl, sc);
            self.template()?;
            return Ok(());
        }
        match keyword(&text) {
            Some(k) => self.push(k, sl, sc),
            None => self.push(TokenKind::Ident(text), sl, sc),
        }
        Ok(())
    }

    // --- cattura del template dopo `render` (porting del lexer del prototipo) ---

    /// Delimita il template HTML contando la profondità dei tag, saltando i buchi
    /// di interpolazione `{...}` e le isole `@start-client … @end-client`.
    fn template(&mut self) -> Result<(), LogyxError> {
        while matches!(self.peek(0), ' ' | '\t' | '\r' | '\n') {
            self.advance();
        }
        if self.peek(0) != '<' {
            return Err(self.err("dopo 'render' è atteso un template che inizia con '<'"));
        }
        let (sl, sc) = (self.line, self.col);
        let start = self.i;
        let mut depth: i32 = 0;
        let mut started = false;
        while self.i < self.src.len() {
            let c = self.peek(0);
            if c == '{' {
                self.skip_braces()?;
                continue;
            }
            if c == '@' && self.matches_at("@start-client") {
                self.skip_island()?;
                continue;
            }
            if c == '<' {
                let (closing, selfclose) = self.consume_tag()?;
                if closing {
                    depth -= 1;
                } else if !selfclose {
                    depth += 1;
                    started = true;
                } else if selfclose && depth == 0 {
                    started = true;
                }
                if started && depth == 0 {
                    break;
                }
                continue;
            }
            self.advance();
        }
        let raw: String = self.src[start..self.i].iter().collect();
        self.push(TokenKind::Template(raw), sl, sc);
        Ok(())
    }

    fn consume_tag(&mut self) -> Result<(bool, bool), LogyxError> {
        self.advance(); // '<'
        let closing = self.peek(0) == '/';
        let mut last = '\0';
        while self.i < self.src.len() && self.peek(0) != '>' {
            last = self.advance();
        }
        if self.i >= self.src.len() {
            return Err(self.err("tag del template non terminato (manca '>')"));
        }
        self.advance(); // '>'
        Ok((closing, last == '/'))
    }

    fn skip_braces(&mut self) -> Result<(), LogyxError> {
        self.advance(); // '{'
        let mut depth = 1;
        while self.i < self.src.len() && depth > 0 {
            let c = self.advance();
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
            }
        }
        if depth > 0 {
            return Err(self.err("interpolazione non terminata nel template (manca '}')"));
        }
        Ok(())
    }

    fn skip_island(&mut self) -> Result<(), LogyxError> {
        self.consume_literal("@start-client");
        while self.i < self.src.len() && !self.matches_at("@end-client") {
            self.advance();
        }
        if self.i >= self.src.len() {
            return Err(self.err("isola client non terminata (manca @end-client)"));
        }
        self.consume_literal("@end-client");
        Ok(())
    }

    fn consume_literal(&mut self, lit: &str) {
        for _ in 0..lit.chars().count() {
            self.advance();
        }
    }

    fn matches_at(&self, lit: &str) -> bool {
        let chars: Vec<char> = lit.chars().collect();
        if self.i + chars.len() > self.src.len() {
            return false;
        }
        chars.iter().enumerate().all(|(k, ch)| self.src[self.i + k] == *ch)
    }

    fn string(&mut self) -> Result<(), LogyxError> {
        let (sl, sc) = (self.line, self.col);
        self.advance(); // apre le virgolette
        let mut parts: Vec<StringPart> = Vec::new();
        let mut buf = String::new();
        loop {
            if self.i >= self.src.len() {
                return Err(self.err("stringa non terminata"));
            }
            let c = self.peek(0);
            if c == '"' {
                self.advance();
                break;
            }
            if c == '\\' {
                self.advance();
                let e = self.advance();
                let repl = match e {
                    'n' => '\n',
                    't' => '\t',
                    '"' => '"',
                    '\\' => '\\',
                    '{' => '{',
                    '}' => '}',
                    other => other,
                };
                buf.push(repl);
                continue;
            }
            if c == '{' {
                self.advance();
                if !buf.is_empty() {
                    parts.push(StringPart::Lit(std::mem::take(&mut buf)));
                }
                let mut expr = String::new();
                while self.i < self.src.len() && self.peek(0) != '}' {
                    expr.push(self.advance());
                }
                if self.i >= self.src.len() {
                    return Err(self.err("interpolazione non terminata (manca '}')"));
                }
                self.advance(); // chiude '}'
                let trimmed = expr.trim().to_string();
                if trimmed.is_empty() {
                    return Err(self.err("interpolazione vuota {}"));
                }
                parts.push(StringPart::Expr(trimmed));
                continue;
            }
            buf.push(self.advance());
        }
        if !buf.is_empty() {
            parts.push(StringPart::Lit(buf));
        }
        self.push(TokenKind::Str(parts), sl, sc);
        Ok(())
    }

    fn operator(&mut self) -> Result<(), LogyxError> {
        let (sl, sc) = (self.line, self.col);
        let two = [self.peek(0), self.peek(1)];
        let two_kind = match two {
            ['-', '>'] => Some(TokenKind::Arrow),
            ['=', '='] => Some(TokenKind::Eq),
            ['!', '='] => Some(TokenKind::Ne),
            ['<', '='] => Some(TokenKind::Le),
            ['>', '='] => Some(TokenKind::Ge),
            ['+', '='] => Some(TokenKind::PlusEq),
            ['-', '='] => Some(TokenKind::MinusEq),
            ['*', '='] => Some(TokenKind::StarEq),
            ['/', '='] => Some(TokenKind::SlashEq),
            ['%', '='] => Some(TokenKind::PercentEq),
            _ => None,
        };
        if let Some(k) = two_kind {
            self.advance();
            self.advance();
            self.push(k, sl, sc);
            return Ok(());
        }
        let c = self.peek(0);
        let single = match c {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            '[' => TokenKind::LBrack,
            ']' => TokenKind::RBrack,
            ',' => TokenKind::Comma,
            ':' => TokenKind::Colon,
            '=' => TokenKind::Assign,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            '?' => TokenKind::Question,
            '|' => TokenKind::Pipe,
            '.' => TokenKind::Dot,
            other => return Err(self.err(&format!("carattere inatteso {:?}", other))),
        };
        self.advance();
        self.push(single, sl, sc);
        Ok(())
    }
}
