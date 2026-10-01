// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parser a discesa ricorsiva: dai token all'AST. Porting del parser del prototipo.

use std::mem::discriminant;

use crate::ast::*;
use crate::error::LogyxError;
use crate::lexer::Lexer;
use crate::token::{StringPart, Token, TokenKind};

pub struct Parser {
    toks: Vec<Token>,
    i: usize,
    file: String,
}

impl Parser {
    pub fn new(toks: Vec<Token>, file: String) -> Self {
        Parser { toks, i: 0, file }
    }

    // --- utilità ---

    fn cur(&self) -> &Token {
        &self.toks[self.i]
    }

    fn kind(&self) -> &TokenKind {
        &self.toks[self.i].kind
    }

    fn kind_at(&self, k: usize) -> &TokenKind {
        self.toks
            .get(self.i + k)
            .map(|t| &t.kind)
            .unwrap_or(&TokenKind::Eof)
    }

    fn is(&self, k: &TokenKind) -> bool {
        discriminant(self.kind()) == discriminant(k)
    }

    fn advance(&mut self) -> Token {
        let t = self.toks[self.i].clone();
        self.i += 1;
        t
    }

    fn expect(&mut self, k: &TokenKind, what: &str) -> Result<Token, LogyxError> {
        if self.is(k) {
            Ok(self.advance())
        } else {
            Err(self.error(&format!("atteso {}, trovato {:?}", what, self.kind())))
        }
    }

    fn error(&self, msg: &str) -> LogyxError {
        let t = self.cur();
        LogyxError::new(format!(
            "{}:{}:{}: errore di sintassi: {}",
            self.file, t.line, t.col, msg
        ))
    }

    fn ident_name(&mut self, what: &str) -> Result<String, LogyxError> {
        if let TokenKind::Ident(s) = self.kind().clone() {
            self.advance();
            Ok(s)
        } else {
            Err(self.error(&format!("atteso {}, trovato {:?}", what, self.kind())))
        }
    }

    // --- programma ---

    pub fn parse(&mut self) -> Result<Vec<Item>, LogyxError> {
        let mut items = Vec::new();
        while !self.is(&TokenKind::Eof) {
            if self.is(&TokenKind::Import) {
                items.push(self.import_item()?);
            } else if self.is(&TokenKind::Fn) {
                items.push(Item::Func(self.function()?));
            } else {
                items.push(Item::Stmt(self.statement()?));
            }
        }
        Ok(items)
    }

    fn import_item(&mut self) -> Result<Item, LogyxError> {
        self.expect(&TokenKind::Import, "import")?;
        let t = self.expect(&TokenKind::Str(vec![]), "percorso del modulo da importare")?;
        if let TokenKind::Str(parts) = t.kind {
            if parts.len() == 1 {
                if let StringPart::Lit(s) = &parts[0] {
                    return Ok(Item::Import(s.clone()));
                }
            }
            return Err(self.error(
                "il percorso di 'import' deve essere una stringa semplice (senza interpolazione)",
            ));
        }
        unreachable!()
    }

    fn function(&mut self) -> Result<Function, LogyxError> {
        self.expect(&TokenKind::Fn, "fn")?;
        let name = self.ident_name("nome di funzione")?;
        self.expect(&TokenKind::LParen, "(")?;
        let mut params = Vec::new();
        if !self.is(&TokenKind::RParen) {
            params.push(self.param()?);
            while self.is(&TokenKind::Comma) {
                self.advance();
                params.push(self.param()?);
            }
        }
        self.expect(&TokenKind::RParen, ")")?;
        let mut ret = None;
        if self.is(&TokenKind::Arrow) {
            self.advance();
            ret = Some(self.type_ref()?);
        }
        let body = self.block()?;
        Ok(Function { name, params, ret, body })
    }

    fn param(&mut self) -> Result<Param, LogyxError> {
        let name = self.ident_name("nome di parametro")?;
        let mut ty = None;
        if self.is(&TokenKind::Colon) {
            self.advance();
            ty = Some(self.type_ref()?);
        }
        Ok(Param { name, ty })
    }

    fn type_ref(&mut self) -> Result<TypeRef, LogyxError> {
        let base = self.type_base()?;
        if self.is(&TokenKind::Pipe) {
            self.advance();
            let w = self.ident_name("'error'")?;
            if w != "error" {
                return Err(self.error(&format!(
                    "dopo '|' nel tipo è atteso 'error', trovato '{}'",
                    w
                )));
            }
            return Ok(format!("{}|error", base));
        }
        Ok(base)
    }

    fn type_base(&mut self) -> Result<TypeRef, LogyxError> {
        if self.is(&TokenKind::LBrack) {
            self.advance();
            let inner = self.type_base()?;
            self.expect(&TokenKind::RBrack, "]")?;
            return Ok(format!("[{}]", inner));
        }
        if self.is(&TokenKind::LBrace) {
            self.advance();
            let k = self.type_base()?;
            self.expect(&TokenKind::Colon, ":")?;
            let v = self.type_base()?;
            self.expect(&TokenKind::RBrace, "}")?;
            return Ok(format!("{{{}: {}}}", k, v));
        }
        if self.is(&TokenKind::Nil) {
            self.advance();
            return Ok("nil".to_string());
        }
        self.ident_name("tipo")
    }

    fn block(&mut self) -> Result<Vec<Stmt>, LogyxError> {
        self.expect(&TokenKind::LBrace, "{")?;
        let mut stmts = Vec::new();
        while !self.is(&TokenKind::RBrace) && !self.is(&TokenKind::Eof) {
            if self.is(&TokenKind::Fn) {
                stmts.push(Stmt::Func(self.function()?));
            } else {
                stmts.push(self.statement()?);
            }
        }
        self.expect(&TokenKind::RBrace, "}")?;
        Ok(stmts)
    }

    // --- istruzioni ---

    fn statement(&mut self) -> Result<Stmt, LogyxError> {
        match self.kind() {
            TokenKind::If => self.if_stmt(),
            TokenKind::While => self.while_stmt(),
            TokenKind::For => self.for_stmt(),
            TokenKind::Return => self.return_stmt(),
            TokenKind::Fail => self.fail_stmt(),
            TokenKind::Match => self.match_stmt(),
            TokenKind::Const => self.const_decl(),
            _ => self.decl_or_expr(),
        }
    }

    fn decl_or_expr(&mut self) -> Result<Stmt, LogyxError> {
        // dichiarazione tipizzata:  IDENT ':' tipo '=' espressione
        if matches!(self.kind(), TokenKind::Ident(_))
            && matches!(self.kind_at(1), TokenKind::Colon)
        {
            let name = self.ident_name("nome")?;
            self.advance(); // ':'
            let _ty = self.type_ref()?;
            self.expect(&TokenKind::Assign, "=")?;
            let value = self.expression()?;
            return Ok(Stmt::Decl { name, value, is_const: false });
        }
        let expr = self.expression()?;
        if self.is(&TokenKind::Assign) {
            self.advance();
            let value = self.expression()?;
            match &expr {
                Expr::Ident(_) | Expr::Index { .. } => Ok(Stmt::Assign { target: expr, value }),
                _ => Err(self.error("assegnazione a un bersaglio non valido")),
            }
        } else {
            Ok(Stmt::Expr(expr))
        }
    }

    fn if_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::If, "if")?;
        let cond = self.expression()?;
        let then_block = self.block()?;
        let mut else_block = None;
        if self.is(&TokenKind::Else) {
            self.advance();
            else_block = Some(if self.is(&TokenKind::If) {
                vec![self.if_stmt()?]
            } else {
                self.block()?
            });
        }
        Ok(Stmt::If { cond, then_block, else_block })
    }

    fn while_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::While, "while")?;
        let cond = self.expression()?;
        let body = self.block()?;
        Ok(Stmt::While { cond, body })
    }

    fn for_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::For, "for")?;
        let var = self.ident_name("variabile di ciclo")?;
        self.expect(&TokenKind::In, "in")?;
        let iterable = self.expression()?;
        let body = self.block()?;
        Ok(Stmt::For { var, iterable, body })
    }

    fn return_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::Return, "return")?;
        if self.is(&TokenKind::RBrace) || self.is(&TokenKind::Eof) {
            return Ok(Stmt::Return(None));
        }
        Ok(Stmt::Return(Some(self.expression()?)))
    }

    fn fail_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::Fail, "fail")?;
        Ok(Stmt::Fail(self.expression()?))
    }

    fn match_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::Match, "match")?;
        let subject = self.expression()?;
        self.expect(&TokenKind::LBrace, "{")?;
        let (mut okv, mut okb, mut errv, mut errb) = (None, None, None, None);
        while !self.is(&TokenKind::RBrace) && !self.is(&TokenKind::Eof) {
            let tag = self.ident_name("ramo 'ok' oppure 'err'")?;
            if tag != "ok" && tag != "err" {
                return Err(self.error(&format!(
                    "in 'match' sono ammessi solo i rami 'ok' e 'err', trovato '{}'",
                    tag
                )));
            }
            let var = self.ident_name("nome della variabile del ramo")?;
            let block = self.block()?;
            if tag == "ok" {
                okv = Some(var);
                okb = Some(block);
            } else {
                errv = Some(var);
                errb = Some(block);
            }
        }
        self.expect(&TokenKind::RBrace, "}")?;
        match (okv, okb, errv, errb) {
            (Some(ok_var), Some(ok_block), Some(err_var), Some(err_block)) => Ok(Stmt::Match {
                subject,
                ok_var,
                ok_block,
                err_var,
                err_block,
            }),
            _ => Err(self.error("'match' richiede entrambi i rami 'ok' e 'err'")),
        }
    }

    fn const_decl(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::Const, "const")?;
        let name = self.ident_name("nome costante")?;
        if self.is(&TokenKind::Colon) {
            self.advance();
            self.type_ref()?;
        }
        self.expect(&TokenKind::Assign, "=")?;
        let value = self.expression()?;
        Ok(Stmt::Decl { name, value, is_const: true })
    }

    // --- espressioni (per precedenza crescente) ---

    pub fn expression(&mut self) -> Result<Expr, LogyxError> {
        self.or_expr()
    }

    fn or_expr(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.and_expr()?;
        while self.is(&TokenKind::Or) {
            self.advance();
            let right = self.and_expr()?;
            left = Expr::Logical { op: LogOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.equality()?;
        while self.is(&TokenKind::And) {
            self.advance();
            let right = self.equality()?;
            left = Expr::Logical { op: LogOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn equality(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.comparison()?;
        loop {
            let op = match self.kind() {
                TokenKind::Eq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                _ => break,
            };
            self.advance();
            let right = self.comparison()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn comparison(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.term()?;
        loop {
            let op = match self.kind() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.term()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.factor()?;
        loop {
            let op = match self.kind() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.factor()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.unary()?;
        loop {
            let op = match self.kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, LogyxError> {
        if self.is(&TokenKind::Not) {
            self.advance();
            return Ok(Expr::Unary { op: UnOp::Not, operand: Box::new(self.unary()?) });
        }
        if self.is(&TokenKind::Minus) {
            self.advance();
            return Ok(Expr::Unary { op: UnOp::Neg, operand: Box::new(self.unary()?) });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, LogyxError> {
        let mut e = self.primary()?;
        loop {
            if self.is(&TokenKind::LParen) {
                self.advance();
                let mut args = Vec::new();
                if !self.is(&TokenKind::RParen) {
                    args.push(self.expression()?);
                    while self.is(&TokenKind::Comma) {
                        self.advance();
                        args.push(self.expression()?);
                    }
                }
                self.expect(&TokenKind::RParen, ")")?;
                e = Expr::Call { callee: Box::new(e), args };
            } else if self.is(&TokenKind::LBrack) {
                self.advance();
                let idx = self.expression()?;
                self.expect(&TokenKind::RBrack, "]")?;
                e = Expr::Index { target: Box::new(e), index: Box::new(idx) };
            } else if self.is(&TokenKind::Question) {
                self.advance();
                e = Expr::Try(Box::new(e));
            } else {
                break;
            }
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, LogyxError> {
        match self.kind().clone() {
            TokenKind::Int(v) => {
                self.advance();
                Ok(Expr::Int(v))
            }
            TokenKind::Float(v) => {
                self.advance();
                Ok(Expr::Float(v))
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil)
            }
            TokenKind::Str(parts) => {
                self.advance();
                self.build_string(&parts)
            }
            TokenKind::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.advance();
                let e = self.expression()?;
                self.expect(&TokenKind::RParen, ")")?;
                Ok(e)
            }
            TokenKind::LBrack => {
                self.advance();
                let mut els = Vec::new();
                if !self.is(&TokenKind::RBrack) {
                    els.push(self.expression()?);
                    while self.is(&TokenKind::Comma) {
                        self.advance();
                        if self.is(&TokenKind::RBrack) {
                            break;
                        }
                        els.push(self.expression()?);
                    }
                }
                self.expect(&TokenKind::RBrack, "]")?;
                Ok(Expr::List(els))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                if !self.is(&TokenKind::RBrace) {
                    pairs.push(self.map_pair()?);
                    while self.is(&TokenKind::Comma) {
                        self.advance();
                        if self.is(&TokenKind::RBrace) {
                            break;
                        }
                        pairs.push(self.map_pair()?);
                    }
                }
                self.expect(&TokenKind::RBrace, "}")?;
                Ok(Expr::Map(pairs))
            }
            other => Err(self.error(&format!("espressione attesa, trovato {:?}", other))),
        }
    }

    fn map_pair(&mut self) -> Result<(Expr, Expr), LogyxError> {
        let key = self.expression()?;
        self.expect(&TokenKind::Colon, ":")?;
        let value = self.expression()?;
        Ok((key, value))
    }

    fn build_string(&self, parts: &[StringPart]) -> Result<Expr, LogyxError> {
        let mut out = Vec::new();
        for p in parts {
            match p {
                StringPart::Lit(s) => out.push(StrPart::Lit(s.clone())),
                StringPart::Expr(src) => {
                    let toks = Lexer::new(src, &self.file).tokenize()?;
                    let mut sub = Parser::new(toks, self.file.clone());
                    let e = sub.expression()?;
                    if !sub.is(&TokenKind::Eof) {
                        return Err(self.error(&format!(
                            "interpolazione con espressione non valida: {{{}}}",
                            src
                        )));
                    }
                    out.push(StrPart::Expr(Box::new(e)));
                }
            }
        }
        Ok(Expr::Str(out))
    }
}
