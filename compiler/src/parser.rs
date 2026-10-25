// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parser a discesa ricorsiva: dai token all'AST. Porting del parser del prototipo.

use std::collections::HashSet;
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

    /// Parsa una singola espressione da sorgente (es. un'interpolazione di template).
    pub fn parse_expression(&mut self) -> Result<Expr, LogyxError> {
        let e = self.expression()?;
        if !self.is(&TokenKind::Eof) {
            return Err(self.error("espressione non valida nell'interpolazione del template"));
        }
        Ok(e)
    }

    pub fn parse(&mut self) -> Result<Vec<Item>, LogyxError> {
        let mut items = Vec::new();
        while !self.is(&TokenKind::Eof) {
            if self.is(&TokenKind::Import) {
                items.push(self.import_item()?);
            } else if self.is(&TokenKind::Record) {
                items.push(Item::Record(self.record_def()?));
            } else if self.is(&TokenKind::Enum) {
                items.push(self.enum_def()?);
            } else if self.is(&TokenKind::Use) {
                items.push(self.use_rust()?);
            } else if self.is(&TokenKind::Extern) {
                items.push(Item::ExternFn(self.extern_fn()?));
            } else if self.is(&TokenKind::Fn) {
                items.push(Item::Func(self.function()?));
            } else if matches!(self.kind(), TokenKind::Ident(s) if s == "route") {
                items.push(self.route_def()?);
            } else {
                items.push(Item::Stmt(self.statement()?));
            }
        }
        Ok(items)
    }

    fn enum_def(&mut self) -> Result<Item, LogyxError> {
        self.expect(&TokenKind::Enum, "enum")?;
        let name = self.ident_name("nome dell'enum")?;
        self.expect(&TokenKind::LBrace, "{")?;
        let mut variants = Vec::new();
        if !self.is(&TokenKind::RBrace) {
            variants.push(self.enum_variant()?);
            while self.is(&TokenKind::Comma) {
                self.advance();
                if self.is(&TokenKind::RBrace) {
                    break;
                }
                variants.push(self.enum_variant()?);
            }
        }
        self.expect(&TokenKind::RBrace, "}")?;
        Ok(Item::Enum { name, variants })
    }

    /// Una variante: `Nome` oppure `Nome(tipo, tipo, …)` (payload posizionale).
    fn enum_variant(&mut self) -> Result<(String, Vec<TypeRef>), LogyxError> {
        let name = self.ident_name("nome di variante")?;
        let mut payload = Vec::new();
        if self.is(&TokenKind::LParen) {
            self.advance();
            if !self.is(&TokenKind::RParen) {
                payload.push(self.type_ref()?);
                while self.is(&TokenKind::Comma) {
                    self.advance();
                    payload.push(self.type_ref()?);
                }
            }
            self.expect(&TokenKind::RParen, ")")?;
        }
        Ok((name, payload))
    }

    fn record_def(&mut self) -> Result<RecordDef, LogyxError> {
        self.expect(&TokenKind::Record, "record")?;
        let name = self.ident_name("nome del record")?;
        self.expect(&TokenKind::LBrace, "{")?;
        let mut fields = Vec::new();
        if !self.is(&TokenKind::RBrace) {
            fields.push(self.record_field()?);
            while self.is(&TokenKind::Comma) {
                self.advance();
                if self.is(&TokenKind::RBrace) {
                    break;
                }
                fields.push(self.record_field()?);
            }
        }
        self.expect(&TokenKind::RBrace, "}")?;
        Ok(RecordDef { name, fields })
    }

    fn record_field(&mut self) -> Result<(String, TypeRef), LogyxError> {
        let fname = self.ident_name("nome del campo")?;
        self.expect(&TokenKind::Colon, ":")?;
        let ftype = self.type_ref()?;
        Ok((fname, ftype))
    }

    fn simple_string(&mut self, what: &str) -> Result<String, LogyxError> {
        let t = self.expect(&TokenKind::Str(vec![]), what)?;
        if let TokenKind::Str(parts) = t.kind {
            if parts.len() == 1 {
                if let StringPart::Lit(s) = &parts[0] {
                    return Ok(s.clone());
                }
            }
        }
        Err(self.error(&format!("{}: è attesa una stringa semplice", what)))
    }

    fn require_ident(&mut self, word: &str, after: &str) -> Result<(), LogyxError> {
        let got = self.ident_name(&format!("'{}'", word))?;
        if got != word {
            return Err(self.error(&format!("dopo '{}' è atteso '{}'", after, word)));
        }
        Ok(())
    }

    fn use_rust(&mut self) -> Result<Item, LogyxError> {
        self.expect(&TokenKind::Use, "use")?;
        self.require_ident("rust", "use")?;
        let crate_name = self.simple_string("nome della crate")?;
        self.expect(&TokenKind::Assign, "=")?;
        let version = self.simple_string("versione della crate")?;
        Ok(Item::UseRust { crate_name, version })
    }

    fn extern_fn(&mut self) -> Result<ExternFn, LogyxError> {
        self.expect(&TokenKind::Extern, "extern")?;
        self.require_ident("rust", "extern")?;
        self.expect(&TokenKind::Fn, "fn")?;
        let name = self.ident_name("nome di funzione")?;
        self.expect(&TokenKind::LParen, "(")?;
        let mut params = Vec::new();
        if !self.is(&TokenKind::RParen) {
            params.push(self.extern_param()?);
            while self.is(&TokenKind::Comma) {
                self.advance();
                params.push(self.extern_param()?);
            }
        }
        self.expect(&TokenKind::RParen, ")")?;
        self.expect(&TokenKind::Arrow, "'->' con il tipo di ritorno")?;
        let ret = self.type_ref()?;
        self.expect(&TokenKind::Assign, "=")?;
        let body = self.simple_string("corpo Rust dell'extern")?;
        Ok(ExternFn { name, params, ret, body })
    }

    fn extern_param(&mut self) -> Result<(String, TypeRef), LogyxError> {
        let p = self.param()?;
        match p.ty {
            Some(t) => Ok((p.name, t)),
            None => Err(self.error("i parametri di una funzione extern richiedono un tipo")),
        }
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

    fn route_def(&mut self) -> Result<Item, LogyxError> {
        self.advance(); // 'route' (identificatore)
        let t = self.expect(&TokenKind::Str(vec![]), "percorso della route")?;
        let path = match t.kind {
            TokenKind::Str(parts) if parts.len() == 1 => match &parts[0] {
                StringPart::Lit(s) => s.clone(),
                _ => {
                    return Err(self.error(
                        "il percorso della route deve essere una stringa semplice (senza interpolazione)",
                    ))
                }
            },
            _ => {
                return Err(self.error(
                    "il percorso della route deve essere una stringa semplice (senza interpolazione)",
                ))
            }
        };
        let body = self.block()?;
        Ok(Item::Route { path, body })
    }

    fn render_stmt(&mut self) -> Result<Stmt, LogyxError> {
        self.expect(&TokenKind::Render, "render")?;
        if let TokenKind::Template(raw) = self.kind().clone() {
            self.advance();
            Ok(Stmt::Render { template: raw })
        } else {
            Err(self.error("dopo 'render' è atteso un template HTML che inizia con '<'"))
        }
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
        // i parametri con valore di default devono stare in coda
        let mut seen_default = false;
        for p in &params {
            if p.default.is_some() {
                seen_default = true;
            } else if seen_default {
                return Err(self.error(
                    "un parametro senza valore di default non può seguirne uno con default",
                ));
            }
        }
        // un parametro varargs può essere solo l'ultimo
        if let Some(pos) = params.iter().position(|p| p.variadic) {
            if pos != params.len() - 1 {
                return Err(self.error("un parametro varargs può essere solo l'ultimo"));
            }
        }
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
        // varargs: `nome: tipo...` (tre punti) — raccoglie gli argomenti in coda in una lista
        let mut variadic = false;
        if self.is(&TokenKind::Dot) {
            self.advance();
            self.expect(&TokenKind::Dot, ".")?;
            self.expect(&TokenKind::Dot, ".")?;
            variadic = true;
        }
        let mut default = None;
        if self.is(&TokenKind::Assign) {
            self.advance();
            default = Some(self.expression()?);
        }
        if variadic && default.is_some() {
            return Err(self.error("un parametro varargs non può avere un valore di default"));
        }
        Ok(Param { name, ty, default, variadic })
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
        if self.is(&TokenKind::LParen) {
            // tipo tupla: (T1, T2, …); con un solo tipo è un raggruppamento
            self.advance();
            let mut ts = Vec::new();
            ts.push(self.type_base()?);
            while self.is(&TokenKind::Comma) {
                self.advance();
                ts.push(self.type_base()?);
            }
            self.expect(&TokenKind::RParen, ")")?;
            return Ok(if ts.len() == 1 {
                ts.pop().unwrap()
            } else {
                format!("({})", ts.join(","))
            });
        }
        if self.is(&TokenKind::Fn) {
            // tipo funzione: fn(T1, T2) -> R
            self.advance();
            self.expect(&TokenKind::LParen, "(")?;
            let mut ps = Vec::new();
            if !self.is(&TokenKind::RParen) {
                ps.push(self.type_base()?);
                while self.is(&TokenKind::Comma) {
                    self.advance();
                    ps.push(self.type_base()?);
                }
            }
            self.expect(&TokenKind::RParen, ")")?;
            self.expect(&TokenKind::Arrow, "'->' con il tipo di ritorno")?;
            let ret = self.type_base()?;
            return Ok(format!("fn({})->{}", ps.join(","), ret));
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
            TokenKind::Render => self.render_stmt(),
            TokenKind::Ident(s) if s == "respond" && !matches!(self.kind_at(1), TokenKind::Assign) => {
                self.advance(); // 'respond'
                let e = self.expression()?;
                Ok(Stmt::Respond(e))
            }
            TokenKind::Break => {
                self.advance();
                Ok(Stmt::Break)
            }
            TokenKind::Continue => {
                self.advance();
                Ok(Stmt::Continue)
            }
            TokenKind::Const => self.const_decl(),
            _ => self.decl_or_expr(),
        }
    }

    fn decl_or_expr(&mut self) -> Result<Stmt, LogyxError> {
        // destructuring di tupla:  IDENT ',' IDENT (',' IDENT)* '=' espressione
        if matches!(self.kind(), TokenKind::Ident(_))
            && matches!(self.kind_at(1), TokenKind::Comma)
        {
            let mut k = 0;
            let is_destructure = loop {
                if !matches!(self.kind_at(k), TokenKind::Ident(_)) {
                    break false;
                }
                match self.kind_at(k + 1) {
                    TokenKind::Comma => k += 2,
                    TokenKind::Assign => break true,
                    _ => break false,
                }
            };
            if is_destructure {
                let mut names = vec![self.ident_name("nome")?];
                while self.is(&TokenKind::Comma) {
                    self.advance();
                    names.push(self.ident_name("nome")?);
                }
                self.expect(&TokenKind::Assign, "=")?;
                let value = self.expression()?;
                return Ok(Stmt::Destructure { names, value });
            }
        }
        // dichiarazione tipizzata:  IDENT ':' tipo '=' espressione
        if matches!(self.kind(), TokenKind::Ident(_))
            && matches!(self.kind_at(1), TokenKind::Colon)
        {
            let name = self.ident_name("nome")?;
            self.advance(); // ':'
            let ty = self.type_ref()?;
            self.expect(&TokenKind::Assign, "=")?;
            let value = self.expression()?;
            return Ok(Stmt::Decl { name, value, is_const: false, ty: Some(ty) });
        }
        let expr = self.expression()?;
        if self.is(&TokenKind::Assign) {
            self.advance();
            let value = self.expression()?;
            return match &expr {
                Expr::Ident(_) | Expr::Index { .. } => Ok(Stmt::Assign { target: expr, value }),
                _ => Err(self.error("assegnazione a un bersaglio non valido")),
            };
        }
        let compound = match self.kind() {
            TokenKind::PlusEq => Some(BinOp::Add),
            TokenKind::MinusEq => Some(BinOp::Sub),
            TokenKind::StarEq => Some(BinOp::Mul),
            TokenKind::SlashEq => Some(BinOp::Div),
            TokenKind::PercentEq => Some(BinOp::Mod),
            _ => None,
        };
        if let Some(op) = compound {
            self.advance();
            let rhs = self.expression()?;
            return match &expr {
                Expr::Ident(_) | Expr::Index { .. } => {
                    let value = Expr::Binary {
                        op,
                        left: Box::new(expr.clone()),
                        right: Box::new(rhs),
                    };
                    Ok(Stmt::Assign { target: expr, value })
                }
                _ => Err(self.error("assegnazione composta a un bersaglio non valido")),
            };
        }
        Ok(Stmt::Expr(expr))
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
        let first = self.ident_name("variabile di ciclo")?;
        let mut index = None;
        let mut var = first.clone();
        if self.is(&TokenKind::Comma) {
            // for indice, elemento in ...
            self.advance();
            index = Some(first);
            var = self.ident_name("variabile di ciclo")?;
        }
        self.expect(&TokenKind::In, "in")?;
        let iterable = self.expression()?;
        let body = self.block()?;
        Ok(Stmt::For { var, iterable, body, index })
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
        let is_result = matches!(self.kind(), TokenKind::Ident(s) if s == "ok" || s == "err");
        if !is_result {
            let mut cases = Vec::new();
            let mut else_block = None;
            while !self.is(&TokenKind::RBrace) && !self.is(&TokenKind::Eof) {
                if self.is(&TokenKind::Else) {
                    self.advance();
                    else_block = Some(self.block()?);
                } else {
                    let pat = self.expression()?;
                    let blk = self.block()?;
                    cases.push((pat, blk));
                }
            }
            self.expect(&TokenKind::RBrace, "}")?;
            // match su enum con payload: se almeno un pattern è `Variante(bind, …)`
            // (chiamata con soli identificatori), è un MatchEnum con binding.
            let is_enum = cases.iter().any(|(p, _)| match p {
                Expr::Call { callee, args } => {
                    matches!(&**callee, Expr::Ident(_))
                        && args.iter().all(|a| matches!(a, Expr::Ident(_)))
                }
                _ => false,
            });
            if is_enum {
                let mut ecases = Vec::new();
                for (pat, block) in cases {
                    let (variant, binds) = match pat {
                        Expr::Call { callee, args } => {
                            let v = match *callee {
                                Expr::Ident(n) => n,
                                _ => return Err(self.error("pattern di variante non valido")),
                            };
                            let mut bs = Vec::new();
                            for a in args {
                                match a {
                                    Expr::Ident(b) => bs.push(b),
                                    _ => return Err(self.error(
                                        "in un pattern di variante gli argomenti devono essere nomi (binding)",
                                    )),
                                }
                            }
                            (v, bs)
                        }
                        Expr::Ident(v) => (v, Vec::new()),
                        _ => return Err(self.error(
                            "in 'match' su enum i rami sono 'Variante(bind…)' o 'Variante'",
                        )),
                    };
                    ecases.push(EnumCase { variant, binds, block });
                }
                return Ok(Stmt::MatchEnum { subject, cases: ecases, else_block });
            }
            return Ok(Stmt::MatchValue { subject, cases, else_block });
        }
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
        let mut ty = None;
        if self.is(&TokenKind::Colon) {
            self.advance();
            ty = Some(self.type_ref()?);
        }
        self.expect(&TokenKind::Assign, "=")?;
        let value = self.expression()?;
        Ok(Stmt::Decl { name, value, is_const: true, ty })
    }

    // --- espressioni (per precedenza crescente) ---

    pub fn expression(&mut self) -> Result<Expr, LogyxError> {
        let cond = self.or_expr()?;
        // espressione condizionale (ternario): `cond ? then : els`
        if self.is(&TokenKind::Question) {
            self.advance();
            let then = self.or_expr()?;
            self.expect(&TokenKind::Colon, "':' nell'espressione condizionale (cond ? a : b)")?;
            let els = self.expression()?; // destra-associativo
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                then: Box::new(then),
                els: Box::new(els),
            });
        }
        Ok(cond)
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
        let mut left = self.bitor()?;
        while self.is(&TokenKind::And) {
            self.advance();
            let right = self.bitor()?;
            left = Expr::Logical { op: LogOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // bitwise: `|` più lasco di `^` più lasco di `&` (sotto l'uguaglianza, come in C)
    fn bitor(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.bitxor()?;
        while self.is(&TokenKind::Pipe) {
            self.advance();
            let right = self.bitxor()?;
            left = Expr::Binary { op: BinOp::BitOr, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn bitxor(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.bitand()?;
        while self.is(&TokenKind::Caret) {
            self.advance();
            let right = self.bitand()?;
            left = Expr::Binary { op: BinOp::BitXor, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn bitand(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.equality()?;
        while self.is(&TokenKind::Amp) {
            self.advance();
            let right = self.equality()?;
            left = Expr::Binary { op: BinOp::BitAnd, left: Box::new(left), right: Box::new(right) };
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
        let mut left = self.shift()?;
        loop {
            // test di appartenenza: `item in coll`
            if self.is(&TokenKind::In) {
                self.advance();
                let coll = self.shift()?;
                left = Expr::In { item: Box::new(left), coll: Box::new(coll) };
                continue;
            }
            let op = match self.kind() {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.shift()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // scorrimenti `<<`/`>>`: fra l'additivo e il confronto
    fn shift(&mut self) -> Result<Expr, LogyxError> {
        let mut left = self.term()?;
        loop {
            let op = match self.kind() {
                TokenKind::Shl => BinOp::Shl,
                TokenKind::Shr => BinOp::Shr,
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
        let mut left = self.power()?;
        loop {
            let op = match self.kind() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.power()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    // esponente `**`, più stretto di `*`/`/`, associativo a destra (2 ** 3 ** 2 = 2 ** 9)
    fn power(&mut self) -> Result<Expr, LogyxError> {
        let base = self.unary()?;
        if self.is(&TokenKind::StarStar) {
            self.advance();
            let exp = self.power()?;
            return Ok(Expr::Binary {
                op: BinOp::Pow,
                left: Box::new(base),
                right: Box::new(exp),
            });
        }
        Ok(base)
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
        if self.is(&TokenKind::Tilde) {
            self.advance();
            return Ok(Expr::Unary { op: UnOp::BitNot, operand: Box::new(self.unary()?) });
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
                // `?` è ternario se seguito da un inizio di espressione SULLA STESSA RIGA;
                // altrimenti è propagazione (gli a-capo non sono token: `x = f()?` seguito
                // da una riga che inizia con un nome non deve diventare un ternario)
                let same_line = self
                    .toks
                    .get(self.i + 1)
                    .map_or(false, |t| t.line == self.cur().line);
                if starts_expr(self.kind_at(1)) && same_line {
                    break;
                }
                self.advance();
                e = Expr::Try(Box::new(e));
            } else if self.is(&TokenKind::Dot) {
                self.advance();
                let name = self.ident_name("nome del campo")?;
                e = Expr::Field { target: Box::new(e), name };
            } else {
                break;
            }
        }
        Ok(e)
    }

    /// Lambda inline: `|p1, p2| espressione`.
    fn lambda(&mut self) -> Result<Expr, LogyxError> {
        self.expect(&TokenKind::Pipe, "|")?;
        let mut params = Vec::new();
        if !self.is(&TokenKind::Pipe) {
            params.push(self.ident_name("parametro della lambda")?);
            while self.is(&TokenKind::Comma) {
                self.advance();
                params.push(self.ident_name("parametro della lambda")?);
            }
        }
        self.expect(&TokenKind::Pipe, "| di chiusura dei parametri")?;
        let body = self.expression()?;
        Ok(Expr::Lambda { params, body: Box::new(body) })
    }

    fn primary(&mut self) -> Result<Expr, LogyxError> {
        match self.kind().clone() {
            TokenKind::Pipe => self.lambda(),
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
                // tupla `(a, b, …)` se c'è una virgola; altrimenti raggruppamento
                if self.is(&TokenKind::Comma) {
                    let mut els = vec![e];
                    while self.is(&TokenKind::Comma) {
                        self.advance();
                        if self.is(&TokenKind::RParen) {
                            break;
                        }
                        els.push(self.expression()?);
                    }
                    self.expect(&TokenKind::RParen, ")")?;
                    return Ok(Expr::Tuple(els));
                }
                self.expect(&TokenKind::RParen, ")")?;
                Ok(e)
            }
            TokenKind::LBrack => {
                self.advance();
                if self.is(&TokenKind::RBrack) {
                    self.advance();
                    return Ok(Expr::List(Vec::new()));
                }
                let first = self.expression()?;
                // list comprehension: `[espr for var in iterabile (if cond)?]`
                if self.is(&TokenKind::For) {
                    self.advance();
                    let var = self.ident_name("variabile della comprehension")?;
                    self.expect(&TokenKind::In, "in")?;
                    let iterable = self.expression()?;
                    let cond = if self.is(&TokenKind::If) {
                        self.advance();
                        Some(Box::new(self.expression()?))
                    } else {
                        None
                    };
                    self.expect(&TokenKind::RBrack, "]")?;
                    return Ok(Expr::Comprehension {
                        expr: Box::new(first),
                        var,
                        iterable: Box::new(iterable),
                        cond,
                    });
                }
                // lista normale
                let mut els = vec![first];
                while self.is(&TokenKind::Comma) {
                    self.advance();
                    if self.is(&TokenKind::RBrack) {
                        break;
                    }
                    els.push(self.expression()?);
                }
                self.expect(&TokenKind::RBrack, "]")?;
                Ok(Expr::List(els))
            }
            TokenKind::LBrace => {
                self.advance();
                // `{}` = mappa vuota; `{k: v, …}` = mappa; `{a, b, …}` = insieme (set)
                if self.is(&TokenKind::RBrace) {
                    self.advance();
                    return Ok(Expr::Map(Vec::new()));
                }
                let first = self.expression()?;
                if self.is(&TokenKind::Colon) {
                    // mappa
                    self.advance();
                    let v = self.expression()?;
                    let mut pairs = vec![(first, v)];
                    while self.is(&TokenKind::Comma) {
                        self.advance();
                        if self.is(&TokenKind::RBrace) {
                            break;
                        }
                        pairs.push(self.map_pair()?);
                    }
                    self.expect(&TokenKind::RBrace, "}")?;
                    Ok(Expr::Map(pairs))
                } else {
                    // insieme
                    let mut els = vec![first];
                    while self.is(&TokenKind::Comma) {
                        self.advance();
                        if self.is(&TokenKind::RBrace) {
                            break;
                        }
                        els.push(self.expression()?);
                    }
                    self.expect(&TokenKind::RBrace, "}")?;
                    Ok(Expr::Set(els))
                }
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

// === Isole client (@start-client … @end-client) → JavaScript (web Fase 2c) ===
//
// Porting di `prototype/logyx/client.py`: compila la DSL client (on/set/if/for/
// while/assegnazioni/espressioni) in JavaScript, riusando il Parser per le
// espressioni. L'output coincide con quello del prototipo (conformità del render).

/// Runtime JS minimo: gli stessi builtin del nucleo disponibili nel client.
const CLIENT_PRELUDE: &str = "function range(n){return Array.from({length: n}, function(_, i){return i;});}\n\
function len(x){return x.length;}\n\
function str(x){return String(x);}\n\
function print(){console.log.apply(console, arguments);}";

/// Compila il corpo di un'isola client in un blocco `<script>` (IIFE).
pub fn compile_island(src: &str, file: &str) -> Result<String, LogyxError> {
    let toks = Lexer::new(src, file).tokenize()?;
    let mut c = ClientCompiler { p: Parser::new(toks, file.to_string()), declared: HashSet::new() };
    let js = c.compile()?;
    Ok(format!(
        "<script>\n(function() {{\n{}\n{}\n}})();\n</script>",
        CLIENT_PRELUDE, js
    ))
}

struct ClientCompiler {
    p: Parser,
    declared: HashSet<String>,
}

impl ClientCompiler {
    fn compile(&mut self) -> Result<String, LogyxError> {
        let mut out = Vec::new();
        while !self.p.is(&TokenKind::Eof) {
            out.push(self.client_stmt()?);
        }
        Ok(out.join("\n"))
    }

    fn client_stmt(&mut self) -> Result<String, LogyxError> {
        match self.p.kind() {
            TokenKind::Ident(s) if s == "on" => self.on_handler(),
            TokenKind::Ident(s) if s == "set" => self.set_stmt(),
            TokenKind::Ident(s) if s == "fetch" => self.client_fetch(),
            TokenKind::If => self.client_if(),
            TokenKind::For => self.client_for(),
            TokenKind::While => self.client_while(),
            TokenKind::Ident(_) if matches!(self.p.kind_at(1), TokenKind::Assign) => {
                let name = self.p.ident_name("nome di variabile")?;
                self.p.advance(); // '='
                let e = self.p.expression()?;
                let js = self.js_expr(&e)?;
                if self.declared.contains(&name) {
                    Ok(format!("{} = {};", name, js))
                } else {
                    self.declared.insert(name.clone());
                    Ok(format!("let {} = {};", name, js))
                }
            }
            _ => {
                let e = self.p.expression()?;
                Ok(format!("{};", self.js_expr(&e)?))
            }
        }
    }

    fn on_handler(&mut self) -> Result<String, LogyxError> {
        self.p.advance(); // 'on'
        let event = self.literal_string()?;
        self.expect_word("of")?;
        let selector = self.literal_string()?;
        let body = self.client_block()?;
        Ok(format!(
            "document.querySelector({}).addEventListener({}, function() {{\n{}\n}});",
            js_string(&selector),
            js_string(&event),
            body
        ))
    }

    fn set_stmt(&mut self) -> Result<String, LogyxError> {
        self.p.advance(); // 'set'
        let prop = self.p.advance();
        let prop_name = match &prop.kind {
            TokenKind::Ident(s) if s == "text" || s == "html" => s.clone(),
            _ => return Err(LogyxError::new("atteso 'text' o 'html' dopo 'set' nell'isola client")),
        };
        self.expect_word("of")?;
        let selector = self.literal_string()?;
        self.expect_word("to")?;
        let e = self.p.expression()?;
        let js = self.js_expr(&e)?;
        let attr = if prop_name == "text" { "textContent" } else { "innerHTML" };
        Ok(format!("document.querySelector({}).{} = {};", js_string(&selector), attr, js))
    }

    fn client_fetch(&mut self) -> Result<String, LogyxError> {
        // fetch <url> [method <espr>] [body <espr>] [as <var>] { ... } [else { ... }]
        self.p.advance(); // 'fetch'
        let ue = self.p.expression()?;
        let url = self.js_expr(&ue)?;
        let mut method: Option<String> = None;
        let mut body: Option<String> = None;
        loop {
            let w = match self.p.kind() {
                TokenKind::Ident(s) if s == "method" || s == "body" => s.clone(),
                _ => break,
            };
            self.p.advance();
            let e = self.p.expression()?;
            let js = self.js_expr(&e)?;
            if w == "method" {
                method = Some(js);
            } else {
                body = Some(js);
            }
        }
        let mut var = "_resp".to_string();
        if matches!(self.p.kind(), TokenKind::Ident(s) if s == "as") {
            self.p.advance();
            var = self.p.ident_name("nome di variabile dopo 'as'")?;
        }
        let success = self.client_block()?;
        let mut els: Option<String> = None;
        if self.p.is(&TokenKind::Else) {
            self.p.advance();
            els = Some(self.client_block()?);
        }
        let mut opts: Vec<String> = Vec::new();
        if let Some(m) = &method {
            opts.push(format!("method: {}", m));
        }
        if let Some(b) = &body {
            if method.is_none() {
                opts.push("method: \"POST\"".to_string());
            }
            opts.push(format!("body: {}", b));
        }
        let call = if opts.is_empty() {
            format!("fetch({})", url)
        } else {
            format!("fetch({}, {{{}}})", url, opts.join(", "))
        };
        let catch = match &els {
            Some(e) => format!(".catch(function() {{\n{}\n}})", e),
            None => String::new(),
        };
        Ok(format!(
            "{}.then(function(r) {{ return r.text(); }}).then(function({}) {{\n{}\n}}){};",
            call, var, success, catch
        ))
    }

    fn client_if(&mut self) -> Result<String, LogyxError> {
        self.p.advance(); // 'if'
        let ce = self.p.expression()?;
        let cond = self.js_expr(&ce)?;
        let mut js = format!("if ({}) {{\n{}\n}}", cond, self.client_block()?);
        if self.p.is(&TokenKind::Else) {
            self.p.advance();
            if self.p.is(&TokenKind::If) {
                js += &format!(" else {}", self.client_if()?);
            } else {
                js += &format!(" else {{\n{}\n}}", self.client_block()?);
            }
        }
        Ok(js)
    }

    fn client_for(&mut self) -> Result<String, LogyxError> {
        self.p.advance(); // 'for'
        let var = self.p.ident_name("variabile di ciclo")?;
        self.p.expect(&TokenKind::In, "in")?;
        let ie = self.p.expression()?;
        let iterable = self.js_expr(&ie)?;
        Ok(format!("for (const {} of {}) {{\n{}\n}}", var, iterable, self.client_block()?))
    }

    fn client_while(&mut self) -> Result<String, LogyxError> {
        self.p.advance(); // 'while'
        let ce = self.p.expression()?;
        let cond = self.js_expr(&ce)?;
        Ok(format!("while ({}) {{\n{}\n}}", cond, self.client_block()?))
    }

    fn client_block(&mut self) -> Result<String, LogyxError> {
        self.p.expect(&TokenKind::LBrace, "{")?;
        let mut out = Vec::new();
        while !self.p.is(&TokenKind::RBrace) && !self.p.is(&TokenKind::Eof) {
            out.push(format!("  {}", self.client_stmt()?));
        }
        self.p.expect(&TokenKind::RBrace, "}")?;
        Ok(out.join("\n"))
    }

    fn expect_word(&mut self, word: &str) -> Result<(), LogyxError> {
        let t = self.p.advance();
        match &t.kind {
            TokenKind::Ident(s) if s == word => Ok(()),
            _ => Err(LogyxError::new(format!("atteso '{}' nell'isola client", word))),
        }
    }

    fn literal_string(&mut self) -> Result<String, LogyxError> {
        let t = self.p.expect(&TokenKind::Str(vec![]), "stringa")?;
        if let TokenKind::Str(parts) = t.kind {
            if parts.len() == 1 {
                if let StringPart::Lit(s) = &parts[0] {
                    return Ok(s.clone());
                }
            }
        }
        Err(LogyxError::new(
            "qui è attesa una stringa semplice (senza interpolazione) nell'isola client",
        ))
    }

    fn js_expr(&self, e: &Expr) -> Result<String, LogyxError> {
        match e {
            Expr::Int(n) => Ok(n.to_string()),
            Expr::Float(x) => Ok(format!("{}", x)),
            Expr::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            Expr::Nil => Ok("null".to_string()),
            Expr::Ident(n) => Ok(n.clone()),
            Expr::Str(parts) => {
                let mut buf = String::from("`");
                for part in parts {
                    match part {
                        StrPart::Lit(s) => buf.push_str(
                            &s.replace('\\', "\\\\").replace('`', "\\`").replace("${", "\\${"),
                        ),
                        StrPart::Expr(e) => {
                            buf.push_str("${");
                            buf.push_str(&self.js_expr(e)?);
                            buf.push('}');
                        }
                    }
                }
                buf.push('`');
                Ok(buf)
            }
            Expr::Unary { op, operand } => {
                let sym = match op {
                    UnOp::Not => "!",
                    UnOp::Neg => "-",
                    UnOp::BitNot => "~",
                };
                Ok(format!("({}{})", sym, self.js_expr(operand)?))
            }
            Expr::Binary { op, left, right } => {
                Ok(format!("({} {} {})", self.js_expr(left)?, js_binop(op), self.js_expr(right)?))
            }
            Expr::Logical { op, left, right } => {
                let o = if *op == LogOp::And { "&&" } else { "||" };
                Ok(format!("({} {} {})", self.js_expr(left)?, o, self.js_expr(right)?))
            }
            Expr::Call { callee, args } => {
                let a: Result<Vec<_>, _> = args.iter().map(|x| self.js_expr(x)).collect();
                Ok(format!("{}({})", self.js_expr(callee)?, a?.join(", ")))
            }
            Expr::Index { target, index } => {
                Ok(format!("{}[{}]", self.js_expr(target)?, self.js_expr(index)?))
            }
            Expr::List(els) | Expr::Tuple(els) => {
                let a: Result<Vec<_>, _> = els.iter().map(|x| self.js_expr(x)).collect();
                Ok(format!("[{}]", a?.join(", ")))
            }
            Expr::Set(els) => {
                let a: Result<Vec<_>, _> = els.iter().map(|x| self.js_expr(x)).collect();
                Ok(format!("new Set([{}])", a?.join(", ")))
            }
            Expr::Comprehension { expr, var, iterable, cond } => {
                let it = self.js_expr(iterable)?;
                let filt = match cond {
                    Some(c) => format!(".filter(function({var}) {{ return {}; }})", self.js_expr(c)?),
                    None => String::new(),
                };
                Ok(format!(
                    "{it}{filt}.map(function({var}) {{ return {}; }})",
                    self.js_expr(expr)?
                ))
            }
            Expr::Ternary { cond, then, els } => Ok(format!(
                "({} ? {} : {})",
                self.js_expr(cond)?,
                self.js_expr(then)?,
                self.js_expr(els)?
            )),
            Expr::In { item, coll } => {
                // set → has; array/stringa → includes; oggetto (mappa) → operatore `in`
                let (i, c) = (self.js_expr(item)?, self.js_expr(coll)?);
                Ok(format!(
                    "({c} instanceof Set ? {c}.has({i}) : (Array.isArray({c})||typeof {c}===\"string\") ? {c}.includes({i}) : ({i} in {c}))"
                ))
            }
            Expr::Map(pairs) => {
                let mut ps = Vec::new();
                for (k, v) in pairs {
                    ps.push(format!("{}: {}", self.js_expr(k)?, self.js_expr(v)?));
                }
                Ok(format!("{{{}}}", ps.join(", ")))
            }
            Expr::Field { .. } | Expr::Try(_) | Expr::Lambda { .. } => {
                Err(LogyxError::new("espressione client non supportata (campo/'?'/lambda)"))
            }
        }
    }
}

/// Un token può iniziare un'espressione? (per distinguere `?` ternario da `?` di propagazione)
fn starts_expr(k: &TokenKind) -> bool {
    matches!(
        k,
        TokenKind::Int(_)
            | TokenKind::Float(_)
            | TokenKind::Str(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Nil
            | TokenKind::Ident(_)
            | TokenKind::LParen
            | TokenKind::LBrack
            | TokenKind::LBrace
            | TokenKind::Minus
            | TokenKind::Not
            | TokenKind::Pipe
    )
}

fn js_binop(op: &BinOp) -> &'static str {
    match op {
        BinOp::Eq => "===",
        BinOp::Ne => "!==",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
    }
}

/// Letterale di stringa JS (per selettori/eventi): doppi apici con escape.
fn js_string(s: &str) -> String {
    format!("{:?}", s)
}
