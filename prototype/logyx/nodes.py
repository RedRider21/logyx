# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Nodi dell'albero sintattico (AST)."""

from dataclasses import dataclass
from typing import Any, List, Optional, Tuple


# --- espressioni ---

@dataclass
class Literal:
    value: Any


@dataclass
class StringLit:
    parts: List[Tuple[str, Any]]  # ("lit", str) | ("expr", nodo)


@dataclass
class Identifier:
    name: str


@dataclass
class Unary:
    op: str
    operand: Any


@dataclass
class Binary:
    op: str
    left: Any
    right: Any


@dataclass
class Logical:
    op: str
    left: Any
    right: Any


@dataclass
class Call:
    callee: Any
    args: List[Any]


@dataclass
class Index:
    target: Any
    index: Any


@dataclass
class ListLit:
    elements: List[Any]


@dataclass
class MapLit:
    pairs: List[Tuple[Any, Any]]


# --- istruzioni ---

@dataclass
class Assign:
    target: Any
    value: Any


@dataclass
class Decl:
    name: str
    value: Any
    is_const: bool = False


@dataclass
class FunctionDef:
    name: str
    params: List[str]
    body: List[Any]
    param_types: Optional[List[Optional[str]]] = None
    ret_type: Optional[str] = None


@dataclass
class If:
    cond: Any
    then_block: List[Any]
    else_block: Optional[List[Any]]


@dataclass
class While:
    cond: Any
    body: List[Any]


@dataclass
class For:
    var: str
    iterable: Any
    body: List[Any]


@dataclass
class Return:
    value: Any


@dataclass
class Break:
    pass


@dataclass
class Continue:
    pass


@dataclass
class ExprStmt:
    expr: Any


@dataclass
class Import:
    path: str


@dataclass
class RecordDef:
    name: str
    fields: List[Tuple[str, str]]  # (nome_campo, tipo)


@dataclass
class UseRust:
    crate: str
    version: str


@dataclass
class ExternFn:
    name: str
    params: List[str]
    param_types: List[str]
    ret_type: str
    body: str  # espressione Rust (usa i parametri e la crate)


@dataclass
class Field:
    target: Any
    name: str


@dataclass
class Try:
    operand: Any  # espressione: propaga l'errore se presente, altrimenti il valore


@dataclass
class Fail:
    value: Any  # espressione (messaggio dell'errore)


@dataclass
class Match:
    subject: Any
    ok_var: str
    ok_block: List[Any]
    err_var: str
    err_block: List[Any]


@dataclass
class MatchValue:
    subject: Any
    cases: List[Tuple[Any, List[Any]]]  # (pattern, blocco)
    else_block: Optional[List[Any]]


@dataclass
class RouteDef:
    path: str
    body: List[Any]


@dataclass
class Render:
    raw: str
