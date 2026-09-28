# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Compila un'isola client Logyx (@start-client ... @end-client) in JavaScript.

DSL client v0:
    <nome> = <espressione>                 stato/assegnazione
    on "<evento>" of "<selettore>" { ... } gestore di evento
    set text of "<selettore>" to <espr>    aggiorna il testo di un elemento
"""

import json

from .tokens import T
from . import nodes as N
from .lexer import Lexer
from .parser import Parser
from .errors import LogyxError

_JS_OP = {
    "==": "===", "!=": "!==",
    "<": "<", "<=": "<=", ">": ">", ">=": ">=",
    "+": "+", "-": "-", "*": "*", "/": "/", "%": "%",
}


class ClientCompiler:
    def __init__(self, src):
        self.p = Parser(Lexer(src, "<client>").tokenize(), "<client>")
        self.declared = set()

    def compile(self):
        stmts = []
        while not self.p.at(T.EOF):
            stmts.append(self.client_stmt())
        return "\n".join(stmts)

    # --- istruzioni client ---

    def client_stmt(self):
        t = self.p.peek()
        if t.type == T.IDENT and t.value == "on":
            return self.on_handler()
        if t.type == T.IDENT and t.value == "set":
            return self.set_text()
        if t.type == T.IDENT and self.p.peek(1).type == T.ASSIGN:
            name = self.p.advance().value
            self.p.advance()  # '='
            js = self.js_expr(self.p.expression())
            if name in self.declared:
                return f"{name} = {js};"
            self.declared.add(name)
            return f"let {name} = {js};"
        return self.js_expr(self.p.expression()) + ";"

    def on_handler(self):
        self.p.advance()  # 'on'
        event = self.literal_string()
        self.expect_word("of")
        selector = self.literal_string()
        body = self.block()
        return (
            f"document.querySelector({json.dumps(selector)})"
            f".addEventListener({json.dumps(event)}, function() {{\n{body}\n}});"
        )

    def set_text(self):
        self.p.advance()  # 'set'
        self.expect_word("text")
        self.expect_word("of")
        selector = self.literal_string()
        self.expect_word("to")
        js = self.js_expr(self.p.expression())
        return f"document.querySelector({json.dumps(selector)}).textContent = {js};"

    def block(self):
        self.p.expect(T.LBRACE)
        out = []
        while not self.p.at(T.RBRACE) and not self.p.at(T.EOF):
            out.append("  " + self.client_stmt())
        self.p.expect(T.RBRACE)
        return "\n".join(out)

    def expect_word(self, word):
        t = self.p.advance()
        if not (t.type == T.IDENT and t.value == word):
            self.p.error(f"atteso '{word}' nell'isola client, trovato {t.value!r}", t)

    def literal_string(self):
        tok = self.p.expect(T.STRING, "stringa")
        parts = tok.value
        if len(parts) != 1 or parts[0][0] != "lit":
            self.p.error("qui e' attesa una stringa semplice (senza interpolazione)", tok)
        return parts[0][1]

    # --- espressioni Logyx -> JavaScript ---

    def js_expr(self, node):
        method = getattr(self, "js_" + type(node).__name__, None)
        if method is None:
            raise LogyxError(f"espressione client non supportata: {type(node).__name__}")
        return method(node)

    def js_Literal(self, n):
        return json.dumps(n.value)

    def js_StringLit(self, n):
        buf = ["`"]
        for kind, val in n.parts:
            if kind == "lit":
                buf.append(val.replace("\\", "\\\\").replace("`", "\\`").replace("${", "\\${"))
            else:
                buf.append("${" + self.js_expr(val) + "}")
        buf.append("`")
        return "".join(buf)

    def js_Identifier(self, n):
        return n.name

    def js_Unary(self, n):
        return ("!" if n.op == "not" else "-") + self.js_expr(n.operand)

    def js_Binary(self, n):
        return f"({self.js_expr(n.left)} {_JS_OP[n.op]} {self.js_expr(n.right)})"

    def js_Logical(self, n):
        op = "&&" if n.op == "and" else "||"
        return f"({self.js_expr(n.left)} {op} {self.js_expr(n.right)})"

    def js_Call(self, n):
        args = ", ".join(self.js_expr(a) for a in n.args)
        return f"{self.js_expr(n.callee)}({args})"

    def js_Index(self, n):
        return f"{self.js_expr(n.target)}[{self.js_expr(n.index)}]"

    def js_ListLit(self, n):
        return "[" + ", ".join(self.js_expr(e) for e in n.elements) + "]"

    def js_MapLit(self, n):
        pairs = ", ".join(f"{self.js_expr(k)}: {self.js_expr(v)}" for k, v in n.pairs)
        return "{" + pairs + "}"


def compile_island(src):
    """Compila il corpo di un'isola client in un blocco <script> (IIFE)."""
    js = ClientCompiler(src).compile()
    return "<script>\n(function() {\n" + js + "\n})();\n</script>"
