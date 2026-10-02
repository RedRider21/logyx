// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Backend: genera codice Rust dall'AST. Porting di `prototype/logyx/rustgen.py`.
//!
//! Supporta un sottoinsieme tipizzato: funzioni, aritmetica, confronti, logica,
//! stringhe e concatenazione, if/while/for (range e liste), ricorsione, liste e
//! mappe di scalari, moduli e gestione errori (`fail`/`?`/`match` -> Result).
//! I tipi di ritorno e dei parametri sono dedotti dall'uso.

use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::LogyxError;

type R<T> = Result<T, LogyxError>;

fn ty(t: &str) -> R<&'static str> {
    match t {
        "int" => Ok("i64"),
        "float" => Ok("f64"),
        "bool" => Ok("bool"),
        "string" => Ok("String"),
        other => Err(LogyxError::new(format!(
            "tipo non supportato dal backend: '{}'",
            other
        ))),
    }
}

fn rust_str_lit(s: &str) -> String {
    // {:?} produce una stringa Rust valida (virgolette ed escape corretti).
    format!("{:?}", s)
}

fn binop_sym(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
    }
}

fn is_cmp(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

fn stringish(e: &Expr) -> bool {
    match e {
        Expr::Str(_) => true,
        Expr::Binary { op: BinOp::Add, left, right } => stringish(left) || stringish(right),
        _ => false,
    }
}

fn value_kind(e: &Expr) -> Option<&'static str> {
    match e {
        Expr::List(_) => Some("list"),
        Expr::Map(_) => Some("map"),
        _ => None,
    }
}

fn resolve_ev(ev: &HashSet<String>) -> Option<String> {
    // priorità: string > float > int/num > bool
    if ev.contains("string") {
        return Some("string".into());
    }
    if ev.contains("float") {
        return Some("float".into());
    }
    if ev.contains("int") || ev.contains("num") {
        return Some("int".into());
    }
    if ev.contains("bool") {
        return Some("bool".into());
    }
    None
}

pub struct Codegen {
    ptype: HashMap<String, HashMap<String, Option<String>>>,
    param_order: HashMap<String, Vec<String>>,
    rets: HashMap<String, Option<String>>,
    fallible: HashMap<String, bool>,
    tmp: usize,
    cur_fallible: bool,
    kinds: HashMap<String, String>,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            ptype: HashMap::new(),
            param_order: HashMap::new(),
            rets: HashMap::new(),
            fallible: HashMap::new(),
            tmp: 0,
            cur_fallible: false,
            kinds: HashMap::new(),
        }
    }

    pub fn generate(&mut self, items: &[Item]) -> R<String> {
        let mut funcs: Vec<Function> = Vec::new();
        for it in items {
            match it {
                Item::Func(f) => funcs.push(f.clone()),
                Item::Import(_) => {} // già espansi dal resolver
                Item::Stmt(_) => {
                    return Err(LogyxError::new(
                        "il backend supporta solo definizioni di funzione \
                         (niente codice a primo livello)",
                    ))
                }
            }
        }
        if !funcs.iter().any(|f| f.name == "main") {
            return Err(LogyxError::new("manca 'fn main()': serve un punto d'ingresso"));
        }
        for f in &funcs {
            let mut m = HashMap::new();
            for p in &f.params {
                m.insert(p.name.clone(), p.ty.clone());
            }
            self.ptype.insert(f.name.clone(), m);
            self.param_order
                .insert(f.name.clone(), f.params.iter().map(|p| p.name.clone()).collect());
            let (rt, mut fal) = match &f.ret {
                Some(r) if r.ends_with("|error") => {
                    (Some(r[..r.len() - "|error".len()].to_string()), true)
                }
                Some(r) => (Some(r.clone()), false),
                None => (None, false),
            };
            if self.body_has_fail_or_try(&f.body) {
                fal = true;
            }
            self.rets.insert(f.name.clone(), rt);
            self.fallible.insert(f.name.clone(), fal);
        }
        self.infer(&funcs);
        let mut out = Vec::new();
        for f in &funcs {
            out.push(self.func(f)?);
        }
        Ok(out.join("\n\n") + "\n")
    }

    // --- fallibilità (presenza di `fail` o `?`) ---

    fn body_has_fail_or_try(&self, stmts: &[Stmt]) -> bool {
        stmts.iter().any(|s| match s {
            Stmt::Fail(_) => true,
            Stmt::Return(Some(e)) => self.expr_has_try(e),
            Stmt::Decl { value, .. } | Stmt::Assign { value, .. } => self.expr_has_try(value),
            Stmt::Expr(e) => self.expr_has_try(e),
            Stmt::If { then_block, else_block, .. } => {
                self.body_has_fail_or_try(then_block)
                    || else_block.as_ref().map_or(false, |b| self.body_has_fail_or_try(b))
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => self.body_has_fail_or_try(body),
            Stmt::Match { subject, ok_block, err_block, .. } => {
                self.expr_has_try(subject)
                    || self.body_has_fail_or_try(ok_block)
                    || self.body_has_fail_or_try(err_block)
            }
            _ => false,
        })
    }

    fn expr_has_try(&self, e: &Expr) -> bool {
        match e {
            Expr::Try(_) => true,
            Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
                self.expr_has_try(left) || self.expr_has_try(right)
            }
            Expr::Unary { operand, .. } => self.expr_has_try(operand),
            Expr::Call { args, .. } => args.iter().any(|a| self.expr_has_try(a)),
            Expr::Index { target, index } => self.expr_has_try(target) || self.expr_has_try(index),
            Expr::Str(parts) => parts.iter().any(|p| match p {
                StrPart::Expr(e) => self.expr_has_try(e),
                _ => false,
            }),
            Expr::List(xs) => xs.iter().any(|x| self.expr_has_try(x)),
            Expr::Map(ps) => ps.iter().any(|(k, v)| self.expr_has_try(k) || self.expr_has_try(v)),
            _ => false,
        }
    }

    // --- inferenza (punto fisso: parametri + tipi di ritorno) ---

    fn infer(&mut self, funcs: &[Function]) {
        loop {
            let mut changed = false;
            if self.infer_params_once(funcs) {
                changed = true;
            }
            if self.infer_returns_once(funcs) {
                changed = true;
            }
            if !changed {
                break;
            }
        }
    }

    fn infer_params_once(&mut self, funcs: &[Function]) -> bool {
        let mut changed = false;
        for f in funcs {
            let pt = self.ptype[&f.name].clone();
            let unknown: Vec<String> = f
                .params
                .iter()
                .filter(|p| pt.get(&p.name).map_or(true, |o| o.is_none()))
                .map(|p| p.name.clone())
                .collect();
            if unknown.is_empty() {
                continue;
            }
            let mut ev: HashMap<String, HashSet<String>> =
                unknown.iter().map(|n| (n.clone(), HashSet::new())).collect();
            self.scan_stmts(&f.body, &pt, &mut ev, &f.name);
            for (p, e) in &ev {
                if let Some(t) = resolve_ev(e) {
                    let slot = self.ptype.get_mut(&f.name).unwrap().get_mut(p).unwrap();
                    if slot.is_none() {
                        *slot = Some(t);
                        changed = true;
                    }
                }
            }
        }
        changed
    }

    fn infer_returns_once(&mut self, funcs: &[Function]) -> bool {
        let mut changed = false;
        for f in funcs {
            if self.rets[&f.name].is_some() {
                continue;
            }
            if let Some(t) = self.infer_func_ret(f) {
                self.rets.insert(f.name.clone(), Some(t));
                changed = true;
            }
        }
        changed
    }

    fn infer_func_ret(&self, f: &Function) -> Option<String> {
        let ptypes = &self.ptype[&f.name];
        let mut values: Vec<&Expr> = Vec::new();
        collect_return_values(&f.body, &mut values);
        if values.is_empty() {
            return Some("__void__".into());
        }
        for v in values {
            if let Some(t) = self.type_of(v, ptypes) {
                return Some(t);
            }
        }
        None
    }

    fn type_of(&self, e: &Expr, ptypes: &HashMap<String, Option<String>>) -> Option<String> {
        match e {
            Expr::Try(inner) => self.type_of(inner, ptypes),
            Expr::Str(_) => Some("string".into()),
            Expr::Int(_) => Some("int".into()),
            Expr::Float(_) => Some("float".into()),
            Expr::Bool(_) => Some("bool".into()),
            Expr::Nil => None,
            Expr::Ident(name) => ptypes.get(name).cloned().flatten(),
            Expr::Unary { op, operand } => match op {
                UnOp::Not => Some("bool".into()),
                UnOp::Neg => self.type_of(operand, ptypes),
            },
            Expr::Logical { .. } => Some("bool".into()),
            Expr::Binary { op, left, right } => {
                if is_cmp(op) {
                    return Some("bool".into());
                }
                let lt = self.type_of(left, ptypes);
                let rt = self.type_of(right, ptypes);
                let s = Some("string".to_string());
                if *op == BinOp::Add && (lt == s || rt == s) {
                    return Some("string".into());
                }
                let f = Some("float".to_string());
                if lt == f || rt == f {
                    return Some("float".into());
                }
                if lt == Some("int".into()) && rt == Some("int".into()) {
                    return Some("int".into());
                }
                None
            }
            Expr::Call { callee, args } => {
                if let Expr::Ident(name) = &**callee {
                    if name == "str" {
                        return Some("string".into());
                    }
                    if name == "len" {
                        return Some("int".into());
                    }
                    if name == "abs" {
                        return args.first().and_then(|a| self.type_of(a, ptypes));
                    }
                    if name == "min" || name == "max" {
                        return args.iter().find_map(|a| self.type_of(a, ptypes));
                    }
                    if name == "upper" || name == "lower" {
                        return Some("string".into());
                    }
                    if name == "contains" {
                        return Some("bool".into());
                    }
                    return self.rets.get(name).cloned().flatten();
                }
                None
            }
            _ => None,
        }
    }

    fn scan_stmts(
        &self,
        stmts: &[Stmt],
        pt: &HashMap<String, Option<String>>,
        ev: &mut HashMap<String, HashSet<String>>,
        fname: &str,
    ) {
        for s in stmts {
            self.scan_stmt(s, pt, ev, fname);
        }
    }

    fn scan_stmt(
        &self,
        s: &Stmt,
        pt: &HashMap<String, Option<String>>,
        ev: &mut HashMap<String, HashSet<String>>,
        fname: &str,
    ) {
        match s {
            Stmt::If { cond, then_block, else_block } => {
                self.cond_bool(cond, ev);
                self.scan_expr(cond, pt, ev);
                self.scan_stmts(then_block, pt, ev, fname);
                if let Some(b) = else_block {
                    self.scan_stmts(b, pt, ev, fname);
                }
            }
            Stmt::While { cond, body } => {
                self.cond_bool(cond, ev);
                self.scan_expr(cond, pt, ev);
                self.scan_stmts(body, pt, ev, fname);
            }
            Stmt::For { iterable, body, .. } => {
                self.scan_expr(iterable, pt, ev);
                self.scan_stmts(body, pt, ev, fname);
            }
            Stmt::Return(Some(e)) => {
                if let (Expr::Ident(n), Some(Some(rt))) = (e, self.rets.get(fname)) {
                    if ev.contains_key(n) && ["int", "float", "bool", "string"].contains(&rt.as_str()) {
                        ev.get_mut(n).unwrap().insert(rt.clone());
                    }
                }
                self.scan_expr(e, pt, ev);
            }
            Stmt::Return(None) => {}
            Stmt::Decl { value, .. } | Stmt::Assign { value, .. } => self.scan_expr(value, pt, ev),
            Stmt::Expr(e) => self.scan_expr(e, pt, ev),
            Stmt::Fail(e) => self.scan_expr(e, pt, ev),
            Stmt::Match { subject, ok_block, err_block, .. } => {
                self.scan_expr(subject, pt, ev);
                self.scan_stmts(ok_block, pt, ev, fname);
                self.scan_stmts(err_block, pt, ev, fname);
            }
            Stmt::Func(_) => {}
        }
    }

    fn cond_bool(&self, cond: &Expr, ev: &mut HashMap<String, HashSet<String>>) {
        if let Expr::Ident(n) = cond {
            if let Some(set) = ev.get_mut(n) {
                set.insert("bool".into());
            }
        }
    }

    fn scan_expr(
        &self,
        e: &Expr,
        pt: &HashMap<String, Option<String>>,
        ev: &mut HashMap<String, HashSet<String>>,
    ) {
        match e {
            Expr::Binary { op, left, right } => {
                self.binary_ev(op, left, right, pt, ev);
                self.scan_expr(left, pt, ev);
                self.scan_expr(right, pt, ev);
            }
            Expr::Logical { left, right, .. } => {
                for side in [left, right] {
                    if let Expr::Ident(n) = &**side {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("bool".into());
                        }
                    }
                }
                self.scan_expr(left, pt, ev);
                self.scan_expr(right, pt, ev);
            }
            Expr::Unary { op, operand } => {
                if let Expr::Ident(n) = &**operand {
                    if let Some(set) = ev.get_mut(n) {
                        set.insert(if *op == UnOp::Not { "bool" } else { "num" }.into());
                    }
                }
                self.scan_expr(operand, pt, ev);
            }
            Expr::Try(inner) => self.scan_expr(inner, pt, ev),
            Expr::Call { callee, args } => {
                self.call_ev(callee, args, ev);
                for a in args {
                    self.scan_expr(a, pt, ev);
                }
            }
            Expr::Str(parts) => {
                for p in parts {
                    if let StrPart::Expr(e) = p {
                        self.scan_expr(e, pt, ev);
                    }
                }
            }
            Expr::Index { target, index } => {
                if let Expr::Ident(n) = &**index {
                    if let Some(set) = ev.get_mut(n) {
                        set.insert("int".into());
                    }
                }
                self.scan_expr(target, pt, ev);
                self.scan_expr(index, pt, ev);
            }
            Expr::List(xs) => {
                for x in xs {
                    self.scan_expr(x, pt, ev);
                }
            }
            Expr::Map(ps) => {
                for (k, v) in ps {
                    self.scan_expr(k, pt, ev);
                    self.scan_expr(v, pt, ev);
                }
            }
            _ => {}
        }
    }

    fn binary_ev(
        &self,
        op: &BinOp,
        l: &Expr,
        r: &Expr,
        pt: &HashMap<String, Option<String>>,
        ev: &mut HashMap<String, HashSet<String>>,
    ) {
        if is_cmp(op) {
            self.compare_ev(l, r, pt, ev);
            self.compare_ev(r, l, pt, ev);
        } else if matches!(op, BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod) {
            self.arith_ev(l, r, pt, ev);
            self.arith_ev(r, l, pt, ev);
        } else if *op == BinOp::Add {
            let lt = self.type_of(l, pt);
            let rt = self.type_of(r, pt);
            let s = Some("string".to_string());
            if lt == s || rt == s || stringish(l) || stringish(r) {
                for side in [l, r] {
                    if let Expr::Ident(n) = side {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("string".into());
                        }
                    }
                }
            } else {
                self.arith_ev(l, r, pt, ev);
                self.arith_ev(r, l, pt, ev);
            }
        }
    }

    fn compare_ev(
        &self,
        x: &Expr,
        other: &Expr,
        pt: &HashMap<String, Option<String>>,
        ev: &mut HashMap<String, HashSet<String>>,
    ) {
        if let Expr::Ident(n) = x {
            if ev.contains_key(n) {
                let ot = self.type_of(other, pt);
                let tag = match ot.as_deref() {
                    Some(t @ ("int" | "float" | "bool" | "string")) => t.to_string(),
                    _ => "num".to_string(),
                };
                ev.get_mut(n).unwrap().insert(tag);
            }
        }
    }

    fn arith_ev(
        &self,
        x: &Expr,
        other: &Expr,
        pt: &HashMap<String, Option<String>>,
        ev: &mut HashMap<String, HashSet<String>>,
    ) {
        if let Expr::Ident(n) = x {
            if ev.contains_key(n) {
                let ot = self.type_of(other, pt);
                let tag = match ot.as_deref() {
                    Some("float") => "float",
                    Some("int") => "int",
                    _ => "num",
                };
                ev.get_mut(n).unwrap().insert(tag.into());
            }
        }
    }

    fn call_ev(
        &self,
        callee: &Expr,
        args: &[Expr],
        ev: &mut HashMap<String, HashSet<String>>,
    ) {
        if let Expr::Ident(name) = callee {
            if name == "range" {
                for a in args {
                    if let Expr::Ident(n) = a {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("int".into());
                        }
                    }
                }
                return;
            }
            if name == "abs" || name == "min" || name == "max" {
                for a in args {
                    if let Expr::Ident(n) = a {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("num".into());
                        }
                    }
                }
                return;
            }
            if name == "upper" || name == "lower" {
                for a in args {
                    if let Expr::Ident(n) = a {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("string".into());
                        }
                    }
                }
                return;
            }
            if name == "remove" {
                if let Some(Expr::Ident(n)) = args.get(1) {
                    if let Some(set) = ev.get_mut(n) {
                        set.insert("int".into());
                    }
                }
                return;
            }
            if let Some(callee_pt) = self.ptype.get(name) {
                // ordine dei parametri: lo ricaviamo dall'ordine di dichiarazione
                // (ricostruito sotto in generate tramite funcs); qui usiamo la mappa
                // con l'ordine salvato in param_order.
                if let Some(order) = self.param_order.get(name) {
                    for (i, a) in args.iter().enumerate() {
                        if i >= order.len() {
                            break;
                        }
                        if let Some(Some(t)) = callee_pt.get(&order[i]) {
                            if let Expr::Ident(n) = a {
                                if let Some(set) = ev.get_mut(n) {
                                    if ["int", "float", "bool", "string"].contains(&t.as_str()) {
                                        set.insert(t.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // --- generazione ---

    fn func(&mut self, f: &Function) -> R<String> {
        let pt = self.ptype[&f.name].clone();
        for p in &f.params {
            if pt.get(&p.name).map_or(true, |o| o.is_none()) {
                return Err(LogyxError::new(format!(
                    "non riesco a inferire il tipo del parametro '{}' di '{}'; \
                     aggiungi l'annotazione '{}: tipo'",
                    p.name, f.name, p.name
                )));
            }
        }
        let mut params = Vec::new();
        for p in &f.params {
            params.push(format!("{}: {}", p.name, ty(pt[&p.name].as_ref().unwrap())?));
        }
        let params = params.join(", ");
        let ret = self.rets[&f.name].clone().ok_or_else(|| {
            LogyxError::new(format!(
                "non riesco a inferire il tipo di ritorno di '{}'; aggiungi l'annotazione '-> tipo'",
                f.name
            ))
        })?;
        self.cur_fallible = self.fallible[&f.name];
        let ret_str = if self.cur_fallible {
            let inner = if ret == "__void__" { "()".to_string() } else { ty(&ret)?.to_string() };
            format!(" -> Result<{}, String>", inner)
        } else if ret == "__void__" {
            String::new()
        } else {
            format!(" -> {}", ty(&ret)?)
        };
        let mut declared: HashSet<String> = f.params.iter().map(|p| p.name.clone()).collect();
        self.kinds.clear();
        let body = self.block(&f.body, &mut declared, 1)?;
        Ok(format!("fn {}({}){} {{\n{}\n}}", f.name, params, ret_str, body))
    }

    fn block(&mut self, stmts: &[Stmt], declared: &mut HashSet<String>, indent: usize) -> R<String> {
        let mut out = Vec::new();
        for s in stmts {
            out.push(self.stmt(s, declared, indent)?);
        }
        Ok(out.join("\n"))
    }

    fn stmt(&mut self, s: &Stmt, declared: &mut HashSet<String>, indent: usize) -> R<String> {
        let pad = "    ".repeat(indent);
        match s {
            Stmt::Return(v) => {
                if self.cur_fallible {
                    let inner = match v {
                        None => "()".to_string(),
                        Some(e) => self.expr(e)?,
                    };
                    Ok(format!("{pad}return Ok({inner});"))
                } else {
                    match v {
                        None => Ok(format!("{pad}return;")),
                        Some(e) => Ok(format!("{pad}return {};", self.expr(e)?)),
                    }
                }
            }
            Stmt::Fail(e) => Ok(format!("{pad}return Err({});", self.expr(e)?)),
            Stmt::Match { subject, ok_var, ok_block, err_var, err_block } => {
                let subj = self.expr(subject)?;
                let mut ok_decl = declared.clone();
                ok_decl.insert(ok_var.clone());
                let mut err_decl = declared.clone();
                err_decl.insert(err_var.clone());
                let okb = self.block(ok_block, &mut ok_decl, indent + 2)?;
                let errb = self.block(err_block, &mut err_decl, indent + 2)?;
                let arm = "    ".repeat(indent + 1);
                Ok(format!(
                    "{pad}match {subj} {{\n{arm}Ok({ok_var}) => {{\n{okb}\n{arm}}}\n{arm}Err({err_var}) => {{\n{errb}\n{arm}}}\n{pad}}}"
                ))
            }
            Stmt::If { cond, then_block, else_block } => {
                let mut out = format!("{pad}if {} {{\n", self.expr(cond)?);
                out += &self.block(then_block, declared, indent + 1)?;
                out += &format!("\n{pad}}}");
                if let Some(eb) = else_block {
                    out += &format!(" else {{\n{}\n{pad}}}", self.block(eb, declared, indent + 1)?);
                }
                Ok(out)
            }
            Stmt::While { cond, body } => Ok(format!(
                "{pad}while {} {{\n{}\n{pad}}}",
                self.expr(cond)?,
                self.block(body, declared, indent + 1)?
            )),
            Stmt::For { var, iterable, body } => self.for_stmt(var, iterable, body, declared, indent),
            Stmt::Decl { name, value, is_const } => {
                declared.insert(name.clone());
                if let Some(k) = value_kind(value) {
                    self.kinds.insert(name.clone(), k.to_string());
                }
                let kw = if *is_const { "let" } else { "let mut" };
                Ok(format!("{pad}{kw} {name} = {};", self.expr(value)?))
            }
            Stmt::Assign { target, value } => {
                let name = match target {
                    Expr::Ident(n) => n.clone(),
                    _ => {
                        return Err(LogyxError::new(
                            "il backend assegna solo a variabili semplici",
                        ))
                    }
                };
                if let Some(k) = value_kind(value) {
                    self.kinds.insert(name.clone(), k.to_string());
                }
                let val = self.expr(value)?;
                if declared.contains(&name) {
                    Ok(format!("{pad}{name} = {val};"))
                } else {
                    declared.insert(name.clone());
                    Ok(format!("{pad}let mut {name} = {val};"))
                }
            }
            Stmt::Expr(e) => {
                if let Expr::Call { callee, args } = e {
                    if let Expr::Ident(n) = &**callee {
                        if n == "print" {
                            return Ok(format!("{pad}{}", self.print_call(args)?));
                        }
                        if n == "push" {
                            return Ok(format!("{pad}{}", self.push_call(args)?));
                        }
                        if n == "remove" {
                            return Ok(format!("{pad}{}", self.remove_call(args)?));
                        }
                    }
                }
                Ok(format!("{pad}{};", self.expr(e)?))
            }
            Stmt::Func(_) => Err(LogyxError::new(
                "il backend non supporta le funzioni annidate",
            )),
        }
    }

    fn for_stmt(
        &mut self,
        var: &str,
        iterable: &Expr,
        body: &[Stmt],
        declared: &mut HashSet<String>,
        indent: usize,
    ) -> R<String> {
        let pad = "    ".repeat(indent);
        declared.insert(var.to_string());
        // range(n) oppure range(a, b)?
        if let Expr::Call { callee, args } = iterable {
            if let Expr::Ident(name) = &**callee {
                if name == "range" && (args.len() == 1 || args.len() == 2) {
                    let (lo, hi) = if args.len() == 1 {
                        ("0i64".to_string(), self.expr(&args[0])?)
                    } else {
                        (self.expr(&args[0])?, self.expr(&args[1])?)
                    };
                    let b = self.block(body, declared, indent + 1)?;
                    return Ok(format!("{pad}for {var} in ({lo})..({hi}) {{\n{b}\n{pad}}}"));
                }
            }
        }
        // iterazione su lista (elementi Copy, per valore)
        let it = self.expr(iterable)?;
        let b = self.block(body, declared, indent + 1)?;
        Ok(format!("{pad}for {var} in ({it}).iter().copied() {{\n{b}\n{pad}}}"))
    }

    fn push_call(&mut self, args: &[Expr]) -> R<String> {
        if args.len() != 2 {
            return Err(LogyxError::new("push accetta due argomenti: push(lista, valore)"));
        }
        let name = match &args[0] {
            Expr::Ident(n) => n.clone(),
            _ => {
                return Err(LogyxError::new(
                    "push richiede una variabile lista come primo argomento",
                ))
            }
        };
        Ok(format!("{}.push({});", name, self.expr(&args[1])?))
    }

    fn remove_call(&mut self, args: &[Expr]) -> R<String> {
        if args.len() != 2 {
            return Err(LogyxError::new("remove accetta due argomenti: remove(lista, indice)"));
        }
        let name = match &args[0] {
            Expr::Ident(n) => n.clone(),
            _ => {
                return Err(LogyxError::new(
                    "remove richiede una variabile lista come primo argomento",
                ))
            }
        };
        Ok(format!("{}.remove(({}) as usize);", name, self.expr(&args[1])?))
    }

    fn print_call(&mut self, args: &[Expr]) -> R<String> {
        if args.len() != 1 {
            return Err(LogyxError::new("print accetta un solo argomento"));
        }
        match &args[0] {
            Expr::Str(parts) => {
                let (fmt, fargs) = self.format(parts)?;
                Ok(format!("println!({}{});", rust_str_lit(&fmt), fargs))
            }
            other => Ok(format!("println!(\"{{}}\", {});", self.expr(other)?)),
        }
    }

    fn format(&mut self, parts: &[StrPart]) -> R<(String, String)> {
        let mut fmt = String::new();
        let mut fargs = String::new();
        for p in parts {
            match p {
                StrPart::Lit(s) => fmt += &s.replace('{', "{{").replace('}', "}}"),
                StrPart::Expr(e) => {
                    fmt += "{}";
                    fargs += &format!(", {}", self.expr(e)?);
                }
            }
        }
        Ok((fmt, fargs))
    }

    fn expr(&mut self, e: &Expr) -> R<String> {
        match e {
            Expr::Int(v) => Ok(format!("{v}i64")),
            Expr::Float(v) => Ok(format!("{v}f64")),
            Expr::Bool(b) => Ok(if *b { "true" } else { "false" }.to_string()),
            Expr::Nil => Err(LogyxError::new("nil non supportato dal backend")),
            Expr::Str(parts) => {
                if parts.iter().all(|p| matches!(p, StrPart::Lit(_))) {
                    let text: String = parts
                        .iter()
                        .map(|p| match p {
                            StrPart::Lit(s) => s.clone(),
                            _ => String::new(),
                        })
                        .collect();
                    Ok(format!("{}.to_string()", rust_str_lit(&text)))
                } else {
                    let (fmt, fargs) = self.format(parts)?;
                    Ok(format!("format!({}{})", rust_str_lit(&fmt), fargs))
                }
            }
            Expr::Ident(name) => Ok(name.clone()),
            Expr::Unary { op, operand } => {
                let s = self.expr(operand)?;
                Ok(format!("{}{}", if *op == UnOp::Not { "!" } else { "-" }, s))
            }
            Expr::Binary { op, left, right } => {
                if *op == BinOp::Add && (stringish(left) || stringish(right)) {
                    Ok(format!(
                        "format!(\"{{}}{{}}\", {}, {})",
                        self.expr(left)?,
                        self.expr(right)?
                    ))
                } else {
                    Ok(format!(
                        "({} {} {})",
                        self.expr(left)?,
                        binop_sym(op),
                        self.expr(right)?
                    ))
                }
            }
            Expr::Logical { op, left, right } => Ok(format!(
                "({} {} {})",
                self.expr(left)?,
                if *op == LogOp::And { "&&" } else { "||" },
                self.expr(right)?
            )),
            Expr::List(els) => {
                if els.is_empty() {
                    return Err(LogyxError::new(
                        "il backend non deduce il tipo di una lista vuota; usa almeno un elemento",
                    ));
                }
                if els.iter().any(stringish) {
                    return Err(LogyxError::new(
                        "il backend gestisce liste di scalari; le liste di stringhe non sono ancora supportate",
                    ));
                }
                let mut parts = Vec::new();
                for x in els {
                    parts.push(self.expr(x)?);
                }
                Ok(format!("vec![{}]", parts.join(", ")))
            }
            Expr::Map(ps) => {
                if ps.is_empty() {
                    return Err(LogyxError::new(
                        "il backend non deduce il tipo di una mappa vuota; usa almeno una coppia",
                    ));
                }
                if ps.iter().any(|(_, v)| stringish(v)) {
                    return Err(LogyxError::new(
                        "il backend gestisce mappe con valori scalari; i valori stringa non sono ancora supportati",
                    ));
                }
                let name = format!("__m{}", self.tmp);
                self.tmp += 1;
                let mut inserts = String::new();
                for (k, v) in ps {
                    inserts += &format!("{name}.insert({}, {}); ", self.expr(k)?, self.expr(v)?);
                }
                Ok(format!(
                    "{{ let mut {name} = std::collections::HashMap::new(); {inserts}{name} }}"
                ))
            }
            Expr::Try(inner) => Ok(format!("({})?", self.expr(inner)?)),
            Expr::Index { target, index } => {
                if let Expr::Ident(n) = &**target {
                    if self.kinds.get(n).map(|s| s.as_str()) == Some("map") {
                        return Ok(format!(
                            "(*{}.get(&({})).unwrap())",
                            self.expr(target)?,
                            self.expr(index)?
                        ));
                    }
                }
                Ok(format!(
                    "{}[({}) as usize]",
                    self.expr(target)?,
                    self.expr(index)?
                ))
            }
            Expr::Call { callee, args } => {
                if let Expr::Ident(name) = &**callee {
                    if name == "print" {
                        return Err(LogyxError::new(
                            "usa print come istruzione, non dentro un'espressione",
                        ));
                    }
                    if name == "push" {
                        return Err(LogyxError::new(
                            "usa push come istruzione, non dentro un'espressione",
                        ));
                    }
                    if name == "remove" {
                        return Err(LogyxError::new(
                            "usa remove come istruzione, non dentro un'espressione",
                        ));
                    }
                    if name == "len" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("len accetta un solo argomento"));
                        }
                        return Ok(format!("(({}).len() as i64)", self.expr(&args[0])?));
                    }
                    if name == "str" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("str accetta un solo argomento"));
                        }
                        return Ok(format!("format!(\"{{}}\", {})", self.expr(&args[0])?));
                    }
                    if name == "abs" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("abs accetta un solo argomento"));
                        }
                        return Ok(format!("({}).abs()", self.expr(&args[0])?));
                    }
                    if name == "min" || name == "max" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(format!("{name} accetta due argomenti")));
                        }
                        return Ok(format!(
                            "({}).{}({})",
                            self.expr(&args[0])?,
                            name,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "upper" || name == "lower" {
                        if args.len() != 1 {
                            return Err(LogyxError::new(format!("{name} accetta un solo argomento")));
                        }
                        let method = if name == "upper" { "to_uppercase" } else { "to_lowercase" };
                        return Ok(format!("({}).{}()", self.expr(&args[0])?, method));
                    }
                    if name == "contains" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "contains accetta due argomenti: contains(lista, valore)",
                            ));
                        }
                        return Ok(format!(
                            "({}).contains(&({}))",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?
                        ));
                    }
                }
                let mut a = Vec::new();
                for arg in args {
                    a.push(self.expr(arg)?);
                }
                Ok(format!("{}({})", self.expr(callee)?, a.join(", ")))
            }
        }
    }
}

fn collect_return_values<'a>(stmts: &'a [Stmt], out: &mut Vec<&'a Expr>) {
    for s in stmts {
        match s {
            Stmt::Return(Some(e)) => out.push(e),
            Stmt::If { then_block, else_block, .. } => {
                collect_return_values(then_block, out);
                if let Some(b) = else_block {
                    collect_return_values(b, out);
                }
            }
            Stmt::While { body, .. } | Stmt::For { body, .. } => collect_return_values(body, out),
            Stmt::Match { ok_block, err_block, .. } => {
                collect_return_values(ok_block, out);
                collect_return_values(err_block, out);
            }
            _ => {}
        }
    }
}
