// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Albero sintattico (AST) di Logyx. Porting dei nodi del prototipo Python.

/// Un tipo è conservato come stringa (gradual typing), come nel prototipo.
/// Un tipo fallibile ha la forma `"int|error"`.
pub type TypeRef = String;

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LogOp {
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    Not,
    Neg,
}

/// Pezzo di una stringa: testo letterale o espressione interpolata.
#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Lit(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Bool(bool),
    Nil,
    Str(Vec<StrPart>),
    Ident(String),
    Unary { op: UnOp, operand: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Logical { op: LogOp, left: Box<Expr>, right: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Index { target: Box<Expr>, index: Box<Expr> },
    Field { target: Box<Expr>, name: String },
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Try(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Decl { name: String, value: Expr, is_const: bool },
    Assign { target: Expr, value: Expr },
    Expr(Expr),
    If { cond: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    For { var: String, iterable: Expr, body: Vec<Stmt> },
    Return(Option<Expr>),
    Break,
    Continue,
    Fail(Expr),
    Match {
        subject: Expr,
        ok_var: String,
        ok_block: Vec<Stmt>,
        err_var: String,
        err_block: Vec<Stmt>,
    },
    MatchValue {
        subject: Expr,
        cases: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
    },
    Func(Function),
    /// `render <template>`: template HTML grezzo (interpolazione risolta in codegen).
    Render { template: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret: Option<TypeRef>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecordDef {
    pub name: String,
    pub fields: Vec<(String, TypeRef)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExternFn {
    pub name: String,
    pub params: Vec<(String, TypeRef)>,
    pub ret: TypeRef,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Func(Function),
    Record(RecordDef),
    UseRust { crate_name: String, version: String },
    ExternFn(ExternFn),
    Import(String),
    Stmt(Stmt),
    /// `route "<path>" { <stmt> … render <template> }` — handler server-side.
    Route { path: String, body: Vec<Stmt> },
    /// `enum Nome { VarA, VarB, … }` — varianti senza payload (v0).
    Enum { name: String, variants: Vec<String> },
}
