# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later

from dataclasses import dataclass
from typing import Any


class T:
    """Tipi di token."""
    # letterali
    INT = "INT"
    FLOAT = "FLOAT"
    STRING = "STRING"
    TRUE = "TRUE"
    FALSE = "FALSE"
    NIL = "NIL"
    IDENT = "IDENT"
    RENDER = "RENDER"
    TEMPLATE = "TEMPLATE"
    # parole chiave
    FN = "FN"
    RETURN = "RETURN"
    IF = "IF"
    ELSE = "ELSE"
    WHILE = "WHILE"
    FOR = "FOR"
    IN = "IN"
    CONST = "CONST"
    IMPORT = "IMPORT"
    FAIL = "FAIL"
    MATCH = "MATCH"
    BREAK = "BREAK"
    CONTINUE = "CONTINUE"
    RECORD = "RECORD"
    USE = "USE"
    EXTERN = "EXTERN"
    AND = "AND"
    OR = "OR"
    NOT = "NOT"
    # punteggiatura e operatori
    LPAREN = "LPAREN"
    RPAREN = "RPAREN"
    LBRACE = "LBRACE"
    RBRACE = "RBRACE"
    LBRACK = "LBRACK"
    RBRACK = "RBRACK"
    COMMA = "COMMA"
    COLON = "COLON"
    ARROW = "ARROW"
    ASSIGN = "ASSIGN"
    PLUS = "PLUS"
    MINUS = "MINUS"
    STAR = "STAR"
    SLASH = "SLASH"
    PERCENT = "PERCENT"
    PLUSEQ = "PLUSEQ"
    MINUSEQ = "MINUSEQ"
    STAREQ = "STAREQ"
    SLASHEQ = "SLASHEQ"
    PERCENTEQ = "PERCENTEQ"
    EQ = "EQ"
    NE = "NE"
    LT = "LT"
    LE = "LE"
    GT = "GT"
    GE = "GE"
    QUESTION = "QUESTION"
    PIPE = "PIPE"
    DOT = "DOT"
    EOF = "EOF"


KEYWORDS = {
    "fn": T.FN, "return": T.RETURN, "if": T.IF, "else": T.ELSE,
    "while": T.WHILE, "for": T.FOR, "in": T.IN, "const": T.CONST,
    "import": T.IMPORT, "fail": T.FAIL, "match": T.MATCH,
    "break": T.BREAK, "continue": T.CONTINUE, "record": T.RECORD,
    "use": T.USE, "extern": T.EXTERN,
    "and": T.AND, "or": T.OR, "not": T.NOT,
    "true": T.TRUE, "false": T.FALSE, "nil": T.NIL,
}


@dataclass
class Token:
    type: str
    value: Any
    line: int
    col: int
