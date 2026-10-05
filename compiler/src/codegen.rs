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

/// Firma di una funzione esportabile verso WASM (tipi numerici).
pub struct ExportSig {
    pub name: String,
    pub params: Vec<(String, String)>, // (nome, tipo Logyx: "int"/"float"/"bool"/"string")
    pub ret: String,                   // tipo Logyx
}

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

const JSON_HELPER: &str = r#"fn __json_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn __json_float(x: f64) -> String {
    if x.is_finite() && x.fract() == 0.0 {
        format!("{}.0", x)
    } else {
        format!("{}", x)
    }
}"#;

fn fn_name(args: &[Expr], n: usize, idx: usize, usage: &str) -> R<String> {
    let base = usage.split('(').next().unwrap_or(usage);
    if args.len() != n {
        return Err(LogyxError::new(format!("{base} accetta {n} argomenti: {usage}")));
    }
    match &args[idx] {
        Expr::Ident(name) => Ok(name.clone()),
        _ => Err(LogyxError::new(format!(
            "{base}: l'argomento funzione deve essere il nome di una funzione definita con 'fn'"
        ))),
    }
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
    records: HashMap<String, Vec<(String, String)>>,
    tmp: usize,
    cur_fallible: bool,
    kinds: HashMap<String, String>,
    cur_types: HashMap<String, Option<String>>,
    // Per le variabili lista: tipo JSON dell'elemento (per `to_json`), es. "int" o "Persona".
    list_elem: HashMap<String, String>,
    uses_json: bool,
    uses_serde: bool,
    pub deps: HashMap<String, String>,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {
            ptype: HashMap::new(),
            param_order: HashMap::new(),
            rets: HashMap::new(),
            fallible: HashMap::new(),
            records: HashMap::new(),
            tmp: 0,
            cur_fallible: false,
            kinds: HashMap::new(),
            cur_types: HashMap::new(),
            list_elem: HashMap::new(),
            uses_json: false,
            uses_serde: false,
            deps: HashMap::new(),
        }
    }

    /// Funzioni fra `names` con firma interamente numerica (int/float/bool),
    /// esportabili verso WASM senza wasm-bindgen. Da chiamare dopo `generate`.
    /// Funzioni esportabili a WASM: firma fatta di tipi `int|float|bool|string`
    /// (Fase 0 solo numerici; Fase 1 aggiunge `string` via memoria lineare).
    pub fn wasm_exports(&self, names: &[String]) -> Vec<ExportSig> {
        let supported = |t: &str| matches!(t, "int" | "float" | "bool" | "string");
        let mut out = Vec::new();
        for name in names {
            if name == "main" {
                continue;
            }
            let (pt, order) = match (self.ptype.get(name), self.param_order.get(name)) {
                (Some(p), Some(o)) => (p, o),
                _ => continue,
            };
            let ret = match self.rets.get(name) {
                Some(Some(r)) if supported(r) => r.clone(),
                _ => continue,
            };
            let mut params = Vec::new();
            let mut ok = true;
            for pn in order {
                match pt.get(pn) {
                    Some(Some(t)) if supported(t) => params.push((pn.clone(), t.clone())),
                    _ => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                out.push(ExportSig { name: name.clone(), params, ret });
            }
        }
        out
    }

    /// Tipo Rust di un tipo Logyx (base o nome di record).
    fn rust_type(&self, t: &str) -> R<String> {
        if self.records.contains_key(t) {
            return Ok(t.to_string());
        }
        Ok(ty(t)?.to_string())
    }

    pub fn generate(&mut self, items: &[Item]) -> R<String> {
        self.uses_json = false;
        self.uses_serde = false;
        self.deps.clear();
        let mut funcs: Vec<Function> = Vec::new();
        let mut externs: Vec<ExternFn> = Vec::new();
        for it in items {
            match it {
                Item::Func(f) => funcs.push(f.clone()),
                Item::Record(r) => {
                    self.records.insert(r.name.clone(), r.fields.clone());
                }
                Item::UseRust { crate_name, version } => {
                    self.deps.insert(crate_name.clone(), format!("\"{}\"", version));
                }
                Item::ExternFn(ex) => externs.push(ex.clone()),
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
        // firme delle funzioni extern (tipi dichiarati, non fallibili)
        for ex in &externs {
            let mut m = HashMap::new();
            for (n, t) in &ex.params {
                m.insert(n.clone(), Some(t.clone()));
            }
            self.ptype.insert(ex.name.clone(), m);
            self.param_order
                .insert(ex.name.clone(), ex.params.iter().map(|(n, _)| n.clone()).collect());
            self.rets.insert(ex.name.clone(), Some(ex.ret.clone()));
            self.fallible.insert(ex.name.clone(), false);
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
        // le funzioni per prime: possono impostare uses_json / uses_serde / deps
        let mut func_defs = Vec::new();
        for f in &funcs {
            func_defs.push(self.func(f)?);
        }
        // funzioni extern (corpo Rust fornito dall'utente)
        let mut extern_defs = Vec::new();
        for ex in &externs {
            let mut ps = Vec::new();
            for (n, t) in &ex.params {
                ps.push(format!("{}: {}", n, self.rust_type(t)?));
            }
            extern_defs.push(format!(
                "fn {}({}) -> {} {{ {} }}",
                ex.name,
                ps.join(", "),
                self.rust_type(&ex.ret)?,
                ex.body
            ));
        }
        // struct dei record (derive condizionale su serde)
        let derive = if self.uses_serde {
            "Clone, PartialEq, Serialize, Deserialize"
        } else {
            "Clone, PartialEq"
        };
        let mut structs = Vec::new();
        let mut rec_names: Vec<String> = self.records.keys().cloned().collect();
        rec_names.sort();
        for name in &rec_names {
            let fields = self.records[name].clone();
            let mut parts = Vec::new();
            for (fname, ftype) in &fields {
                parts.push(format!("{}: {}", fname, self.rust_type(ftype)?));
            }
            structs.push(format!(
                "#[derive({})]\nstruct {} {{ {} }}",
                derive,
                name,
                parts.join(", ")
            ));
        }
        let mut out = Vec::new();
        if self.uses_serde {
            out.push("use serde::{Serialize, Deserialize};".to_string());
        }
        if self.uses_json {
            out.push(JSON_HELPER.to_string());
        }
        out.extend(structs);
        out.extend(extern_defs);
        out.extend(func_defs);
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
            Stmt::MatchValue { subject, cases, else_block } => {
                self.expr_has_try(subject)
                    || cases.iter().any(|(_, blk)| self.body_has_fail_or_try(blk))
                    || else_block.as_ref().map_or(false, |b| self.body_has_fail_or_try(b))
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
            Expr::Field { target, .. } => self.expr_has_try(target),
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
        // tipi dei parametri + tipi dedotti delle variabili locali
        let mut types = self.ptype[&f.name].clone();
        self.collect_local_types(&f.body, &mut types);
        let mut values: Vec<&Expr> = Vec::new();
        collect_return_values(&f.body, &mut values);
        if values.is_empty() {
            return Some("__void__".into());
        }
        for v in values {
            if let Some(t) = self.type_of(v, &types) {
                return Some(t);
            }
        }
        None
    }

    fn collect_local_types(&self, stmts: &[Stmt], types: &mut HashMap<String, Option<String>>) {
        for s in stmts {
            match s {
                Stmt::Decl { name, value, .. } => {
                    if let Some(vt) = self.type_of(value, types) {
                        types.insert(name.clone(), Some(vt));
                    }
                }
                Stmt::Assign { target: Expr::Ident(n), value } => {
                    if let Some(vt) = self.type_of(value, types) {
                        types.insert(n.clone(), Some(vt));
                    }
                }
                Stmt::If { then_block, else_block, .. } => {
                    self.collect_local_types(then_block, types);
                    if let Some(eb) = else_block {
                        self.collect_local_types(eb, types);
                    }
                }
                Stmt::While { body, .. } | Stmt::For { body, .. } => {
                    self.collect_local_types(body, types)
                }
                Stmt::Match { ok_block, err_block, .. } => {
                    self.collect_local_types(ok_block, types);
                    self.collect_local_types(err_block, types);
                }
                Stmt::MatchValue { cases, else_block, .. } => {
                    for (_, blk) in cases {
                        self.collect_local_types(blk, types);
                    }
                    if let Some(eb) = else_block {
                        self.collect_local_types(eb, types);
                    }
                }
                _ => {}
            }
        }
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
                    if name == "abs" || name == "pow" {
                        return args.first().and_then(|a| self.type_of(a, ptypes));
                    }
                    if name == "min" || name == "max" {
                        return args.iter().find_map(|a| self.type_of(a, ptypes));
                    }
                    if name == "upper" || name == "lower" || name == "trim" || name == "substring"
                        || name == "join" || name == "replace" || name == "to_json"
                        || name == "sha256" || name == "format_date"
                    {
                        return Some("string".into());
                    }
                    if name == "now" {
                        return Some("int".into());
                    }
                    if name == "contains" || name == "has"
                        || name == "starts_with" || name == "ends_with"
                    {
                        return Some("bool".into());
                    }
                    if name == "index_of" {
                        return Some("int".into());
                    }
                    if name == "sum" {
                        return Some("int".into());
                    }
                    if name == "floor" || name == "ceil" {
                        return Some("int".into());
                    }
                    if name == "sqrt" {
                        return Some("float".into());
                    }
                    if name == "reduce" {
                        return args.get(1).and_then(|a| self.type_of(a, ptypes));
                    }
                    if self.records.contains_key(name) {
                        return Some(name.clone());
                    }
                    if name == "from_json" {
                        if let Some(Expr::Ident(r)) = args.get(1) {
                            if self.records.contains_key(r) {
                                return Some(r.clone());
                            }
                        }
                        return None;
                    }
                    return self.rets.get(name).cloned().flatten();
                }
                None
            }
            Expr::Field { target, name } => {
                let tt = self.type_of(target, ptypes)?;
                self.records
                    .get(&tt)
                    .and_then(|fields| fields.iter().find(|(f, _)| f == name).map(|(_, t)| t.clone()))
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
            Stmt::MatchValue { subject, cases, else_block } => {
                for (pat, blk) in cases {
                    self.compare_ev(subject, pat, pt, ev);
                    self.compare_ev(pat, subject, pt, ev);
                    self.scan_expr(pat, pt, ev);
                    self.scan_stmts(blk, pt, ev, fname);
                }
                self.scan_expr(subject, pt, ev);
                if let Some(eb) = else_block {
                    self.scan_stmts(eb, pt, ev, fname);
                }
            }
            Stmt::Break | Stmt::Continue | Stmt::Func(_) => {}
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
            Expr::Field { target, .. } => self.scan_expr(target, pt, ev),
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
            if name == "abs" || name == "min" || name == "max" || name == "pow"
                || name == "floor" || name == "ceil" || name == "sqrt"
            {
                for a in args {
                    if let Expr::Ident(n) = a {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("num".into());
                        }
                    }
                }
                return;
            }
            if name == "upper" || name == "lower" || name == "trim" || name == "index_of"
                || name == "split" || name == "replace"
                || name == "starts_with" || name == "ends_with"
            {
                for a in args {
                    if let Expr::Ident(n) = a {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("string".into());
                        }
                    }
                }
                return;
            }
            if name == "join" {
                if let Some(Expr::Ident(n)) = args.get(1) {
                    if let Some(set) = ev.get_mut(n) {
                        set.insert("string".into());
                    }
                }
                return;
            }
            if name == "substring" {
                if let Some(Expr::Ident(n)) = args.first() {
                    if let Some(set) = ev.get_mut(n) {
                        set.insert("string".into());
                    }
                }
                for a in args.iter().skip(1).take(2) {
                    if let Expr::Ident(n) = a {
                        if let Some(set) = ev.get_mut(n) {
                            set.insert("int".into());
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
            params.push(format!("{}: {}", p.name, self.rust_type(pt[&p.name].as_ref().unwrap())?));
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
            let inner = if ret == "__void__" { "()".to_string() } else { self.rust_type(&ret)? };
            format!(" -> Result<{}, String>", inner)
        } else if ret == "__void__" {
            String::new()
        } else {
            format!(" -> {}", self.rust_type(&ret)?)
        };
        let mut declared: HashSet<String> = f.params.iter().map(|p| p.name.clone()).collect();
        self.kinds.clear();
        self.list_elem.clear();
        self.cur_types = pt.clone();
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
            Stmt::Break => Ok(format!("{pad}break;")),
            Stmt::Continue => Ok(format!("{pad}continue;")),
            Stmt::Fail(e) => Ok(format!("{pad}return Err({});", self.expr(e)?)),
            Stmt::MatchValue { subject, cases, else_block } => {
                let subj = self.expr(subject)?;
                let mut out = String::new();
                for (i, (pat, blk)) in cases.iter().enumerate() {
                    let body = self.block(blk, declared, indent + 1)?;
                    let cond = format!("({} == {})", subj, self.expr(pat)?);
                    if i == 0 {
                        out = format!("{pad}if {cond} {{\n{body}\n{pad}}}");
                    } else {
                        out += &format!(" else if {cond} {{\n{body}\n{pad}}}");
                    }
                }
                if let Some(eb) = else_block {
                    let body = self.block(eb, declared, indent + 1)?;
                    if cases.is_empty() {
                        out = format!("{pad}{{\n{body}\n{pad}}}");
                    } else {
                        out += &format!(" else {{\n{body}\n{pad}}}");
                    }
                }
                if out.is_empty() {
                    out = format!("{pad}{{}}");
                }
                Ok(out)
            }
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
                self.track_list_elem(name, value);
                if let Some(vt) = self.type_of(value, &self.cur_types.clone()) {
                    self.cur_types.insert(name.clone(), Some(vt));
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
                self.track_list_elem(&name, value);
                if let Some(vt) = self.type_of(value, &self.cur_types.clone()) {
                    self.cur_types.insert(name.clone(), Some(vt));
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
                        if n == "sort" {
                            return Ok(format!("{pad}{}", self.sort_call(args)?));
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
        // iterazione su lista (per valore; cloned() vale per scalari e per String)
        let it = self.expr(iterable)?;
        let b = self.block(body, declared, indent + 1)?;
        Ok(format!("{pad}for {var} in ({it}).iter().cloned() {{\n{b}\n{pad}}}"))
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

    fn sort_call(&mut self, args: &[Expr]) -> R<String> {
        if args.len() != 1 {
            return Err(LogyxError::new("sort accetta un solo argomento: sort(lista)"));
        }
        match &args[0] {
            Expr::Ident(n) => Ok(format!("{}.sort();", n)),
            _ => Err(LogyxError::new(
                "sort richiede una variabile lista come argomento",
            )),
        }
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

    fn json_value(&self, rust_expr: &str, type_str: &str) -> R<String> {
        match type_str {
            "int" | "bool" => Ok(format!("format!(\"{{}}\", ({}))", rust_expr)),
            "float" => Ok(format!("__json_float(({}))", rust_expr)),
            "string" => Ok(format!("__json_str(&({}))", rust_expr)),
            t if t.starts_with("list<") && t.ends_with('>') => {
                let elem = &t[5..t.len() - 1];
                let elem_json = self.json_value("__x.clone()", elem)?;
                Ok(format!(
                    "{{ let __items: Vec<String> = ({}).iter().map(|__x| {}).collect(); \
                     format!(\"[{{}}]\", __items.join(\",\")) }}",
                    rust_expr, elem_json
                ))
            }
            t if self.records.contains_key(t) => {
                let fields = self.records[t].clone();
                let mut fmt_parts = Vec::new();
                let mut vals = Vec::new();
                for (fname, ftype) in &fields {
                    fmt_parts.push(format!("\\\"{}\\\":{{}}", fname));
                    vals.push(self.json_value(&format!("({}).{}", rust_expr, fname), ftype)?);
                }
                let mut fmt = String::from("{{");
                fmt.push_str(&fmt_parts.join(","));
                fmt.push_str("}}");
                Ok(format!("format!(\"{}\", {})", fmt, vals.join(", ")))
            }
            other => Err(LogyxError::new(format!(
                "to_json non supporta il tipo '{}' (v0: int, float, bool, string, record, liste)",
                other
            ))),
        }
    }

    /// Tipo JSON di un'espressione passata a `to_json`: i tipi noti via `type_of`,
    /// più le liste (letterali o variabili) come `list<ELEM>`.
    fn json_type_of(&self, e: &Expr) -> Option<String> {
        if let Some(t) = self.type_of(e, &self.cur_types) {
            return Some(t);
        }
        match e {
            Expr::List(els) => {
                let first = els.first()?;
                let elem = self.json_type_of(first)?;
                Some(format!("list<{}>", elem))
            }
            Expr::Ident(n) => self.list_elem.get(n).map(|el| format!("list<{}>", el)),
            _ => None,
        }
    }

    /// Se `value` è una lista letterale non vuota, ricorda il tipo del suo elemento
    /// per la variabile `name` (usato da `to_json`).
    fn track_list_elem(&mut self, name: &str, value: &Expr) {
        if let Expr::List(els) = value {
            if let Some(first) = els.first() {
                if let Some(el) = self.json_type_of(first) {
                    self.list_elem.insert(name.to_string(), el);
                }
            }
        }
    }

    fn arg(&mut self, a: &Expr) -> R<String> {
        // Una variabile passata a una funzione la muove; cloniamo per riusabilità
        // (gratuito per i tipi Copy dopo l'ottimizzazione).
        match a {
            Expr::Ident(_) => Ok(format!("{}.clone()", self.expr(a)?)),
            _ => self.expr(a),
        }
    }

    /// Un'espressione è una stringa? Riconosce i letterali (`stringish`) e, grazie
    /// all'ambiente dei tipi correnti, anche le variabili/espressioni di tipo `string`
    /// (così `r = r + testo` fra `String` genera `format!`, non `r + testo`).
    fn is_string_expr(&self, e: &Expr) -> bool {
        stringish(e) || self.type_of(e, &self.cur_types) == Some("string".to_string())
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
                if *op == BinOp::Add && (self.is_string_expr(left) || self.is_string_expr(right)) {
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
                            "{}.get(&({})).unwrap().clone()",
                            self.expr(target)?,
                            self.expr(index)?
                        ));
                    }
                }
                Ok(format!(
                    "{}[({}) as usize].clone()",
                    self.expr(target)?,
                    self.expr(index)?
                ))
            }
            Expr::Field { target, name } => {
                Ok(format!("{}.{}.clone()", self.expr(target)?, name))
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
                    if name == "has" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "has accetta due argomenti: has(mappa, chiave)",
                            ));
                        }
                        return Ok(format!(
                            "({}).contains_key(&({}))",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "trim" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("trim accetta un solo argomento"));
                        }
                        return Ok(format!("({}).trim().to_string()", self.expr(&args[0])?));
                    }
                    if name == "pow" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "pow accetta due argomenti: pow(base, esponente)",
                            ));
                        }
                        return Ok(format!(
                            "({}).pow(({}) as u32)",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "sum" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("sum accetta un solo argomento"));
                        }
                        return Ok(format!("({}).iter().sum::<i64>()", self.expr(&args[0])?));
                    }
                    if name == "floor" || name == "ceil" {
                        if args.len() != 1 {
                            return Err(LogyxError::new(format!("{name} accetta un solo argomento")));
                        }
                        return Ok(format!("(({}) as f64).{}() as i64", self.expr(&args[0])?, name));
                    }
                    if name == "sqrt" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("sqrt accetta un solo argomento"));
                        }
                        return Ok(format!("(({}) as f64).sqrt()", self.expr(&args[0])?));
                    }
                    if name == "index_of" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "index_of accetta due argomenti: index_of(stringa, sottostringa)",
                            ));
                        }
                        return Ok(format!(
                            "({}).find(({}).as_str()).map(|i| i as i64).unwrap_or(-1i64)",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "substring" {
                        if args.len() != 3 {
                            return Err(LogyxError::new(
                                "substring accetta tre argomenti: substring(stringa, inizio, fine)",
                            ));
                        }
                        return Ok(format!(
                            "({})[({}) as usize..({}) as usize].to_string()",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?,
                            self.expr(&args[2])?
                        ));
                    }
                    if name == "split" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "split accetta due argomenti: split(stringa, separatore)",
                            ));
                        }
                        return Ok(format!(
                            "({}).split(({}).as_str()).map(|x| x.to_string()).collect::<Vec<String>>()",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "join" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "join accetta due argomenti: join(lista, separatore)",
                            ));
                        }
                        return Ok(format!(
                            "({}).join(({}).as_str())",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "replace" {
                        if args.len() != 3 {
                            return Err(LogyxError::new(
                                "replace accetta tre argomenti: replace(stringa, da, a)",
                            ));
                        }
                        return Ok(format!(
                            "({}).replace(({}).as_str(), ({}).as_str())",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?,
                            self.expr(&args[2])?
                        ));
                    }
                    if name == "starts_with" || name == "ends_with" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(format!(
                                "{name} accetta due argomenti: {name}(stringa, parte)"
                            )));
                        }
                        return Ok(format!(
                            "({}).{}(({}).as_str())",
                            self.expr(&args[0])?,
                            name,
                            self.expr(&args[1])?
                        ));
                    }
                    if name == "now" {
                        if !args.is_empty() {
                            return Err(LogyxError::new("now non accetta argomenti"));
                        }
                        return Ok("(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)".to_string());
                    }
                    if name == "format_date" {
                        if args.len() != 1 {
                            return Err(LogyxError::new(
                                "format_date accetta un solo argomento (timestamp)",
                            ));
                        }
                        self.deps.insert("chrono".to_string(), "\"0.4\"".to_string());
                        return Ok(format!(
                            "chrono::DateTime::from_timestamp(({}), 0).unwrap().format(\"%Y-%m-%d %H:%M:%S\").to_string()",
                            self.expr(&args[0])?
                        ));
                    }
                    if name == "from_json" {
                        if args.len() != 2 {
                            return Err(LogyxError::new(
                                "from_json richiede: from_json(testo, NomeRecord)",
                            ));
                        }
                        let rec = match &args[1] {
                            Expr::Ident(n) if self.records.contains_key(n) => n.clone(),
                            _ => {
                                return Err(LogyxError::new(
                                    "from_json: il secondo argomento deve essere il nome di un record",
                                ))
                            }
                        };
                        self.uses_serde = true;
                        self.deps.insert(
                            "serde".to_string(),
                            "{ version = \"1\", features = [\"derive\"] }".to_string(),
                        );
                        self.deps.insert("serde_json".to_string(), "\"1\"".to_string());
                        return Ok(format!(
                            "serde_json::from_str::<{}>(&({})).unwrap()",
                            rec,
                            self.expr(&args[0])?
                        ));
                    }
                    if name == "sha256" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("sha256 accetta un solo argomento"));
                        }
                        self.deps.insert("sha2".to_string(), "\"0.10\"".to_string());
                        return Ok(format!(
                            "{{ use sha2::{{Sha256, Digest}}; let mut __h = Sha256::new(); __h.update(({}).as_bytes()); format!(\"{{:x}}\", __h.finalize()) }}",
                            self.expr(&args[0])?
                        ));
                    }
                    if name == "to_json" {
                        if args.len() != 1 {
                            return Err(LogyxError::new("to_json accetta un solo argomento"));
                        }
                        let ta = self.json_type_of(&args[0]).ok_or_else(|| {
                            LogyxError::new("to_json: non riesco a dedurre il tipo dell'argomento")
                        })?;
                        self.uses_json = true;
                        let ex = self.expr(&args[0])?;
                        return self.json_value(&ex, &ta);
                    }
                    if name == "map" {
                        let f = fn_name(args, 2, 1, "map(lista, funzione)")?;
                        return Ok(format!(
                            "({}).iter().cloned().map(|x| {}(x)).collect::<Vec<_>>()",
                            self.expr(&args[0])?,
                            f
                        ));
                    }
                    if name == "filter" {
                        let f = fn_name(args, 2, 1, "filter(lista, funzione)")?;
                        return Ok(format!(
                            "({}).iter().cloned().filter(|x| {}(x.clone())).collect::<Vec<_>>()",
                            self.expr(&args[0])?,
                            f
                        ));
                    }
                    if name == "reduce" {
                        let f = fn_name(args, 3, 2, "reduce(lista, iniziale, funzione)")?;
                        return Ok(format!(
                            "({}).iter().cloned().fold({}, |acc, x| {}(acc, x))",
                            self.expr(&args[0])?,
                            self.expr(&args[1])?,
                            f
                        ));
                    }
                    if name == "sort" {
                        return Err(LogyxError::new(
                            "usa sort come istruzione, non dentro un'espressione",
                        ));
                    }
                    // costruzione di un record: Nome(v1, v2, ...) posizionale
                    if let Some(fields) = self.records.get(name).cloned() {
                        if args.len() != fields.len() {
                            return Err(LogyxError::new(format!(
                                "il record '{}' ha {} campi, forniti {}",
                                name,
                                fields.len(),
                                args.len()
                            )));
                        }
                        let mut parts = Vec::new();
                        for ((fname, _), arg) in fields.iter().zip(args.iter()) {
                            parts.push(format!("{}: {}", fname, self.arg(arg)?));
                        }
                        return Ok(format!("{} {{ {} }}", name, parts.join(", ")));
                    }
                }
                let mut a = Vec::new();
                for arg in args {
                    a.push(self.arg(arg)?);
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
            Stmt::MatchValue { cases, else_block, .. } => {
                for (_, blk) in cases {
                    collect_return_values(blk, out);
                }
                if let Some(eb) = else_block {
                    collect_return_values(eb, out);
                }
            }
            _ => {}
        }
    }
}
