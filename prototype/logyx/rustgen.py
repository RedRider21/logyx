# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Transpiler v0: da un sottoinsieme tipizzato di Logyx a codice Rust.

Supporta: funzioni con tipi espliciti (int/float/bool/string), aritmetica,
confronti e logica, if/else, while, for su range(n), return, print, ricorsione.
Non supporta ancora liste/mappe, route/render o codice dinamico: in quei casi
solleva un errore chiaro. Serve a dimostrare il percorso di compilazione nativa.
"""

import json

from . import nodes as N
from .errors import LogyxError

_TYPES = {"int": "i64", "float": "f64", "bool": "bool", "string": "String"}


class RustTranspiler:
    def transpile(self, items):
        funcs = [i for i in items if isinstance(i, N.FunctionDef)]
        others = [i for i in items if not isinstance(i, N.FunctionDef)]
        if others:
            raise LogyxError(
                "il transpiler Rust v0 supporta solo definizioni di funzione "
                "(niente route/render o codice a primo livello)"
            )
        if not any(f.name == "main" for f in funcs):
            raise LogyxError("manca 'fn main()': serve un punto d'ingresso")
        return "\n\n".join(self.func(f) for f in funcs) + "\n"

    def func(self, f):
        ptypes = f.param_types or [None] * len(f.params)
        params = ", ".join(f"{n}: {self.ty(t)}" for n, t in zip(f.params, ptypes))
        ret = f" -> {self.ty(f.ret_type)}" if f.ret_type else ""
        declared = set(f.params)
        body = self.block(f.body, declared, 1)
        return f"fn {f.name}({params}){ret} {{\n{body}\n}}"

    def ty(self, t):
        if t is None:
            raise LogyxError("il transpiler v0 richiede tipi espliciti su parametri e tipo di ritorno")
        if t not in _TYPES:
            raise LogyxError(f"tipo non supportato dal transpiler v0: '{t}'")
        return _TYPES[t]

    # --- istruzioni ---

    def block(self, stmts, declared, indent):
        return "\n".join(self.stmt(s, declared, indent) for s in stmts)

    def stmt(self, s, declared, indent):
        pad = "    " * indent
        t = type(s).__name__
        if t == "Return":
            return pad + ("return;" if s.value is None else f"return {self.expr(s.value)};")
        if t == "If":
            out = pad + f"if {self.expr(s.cond)} {{\n"
            out += self.block(s.then_block, declared, indent + 1) + "\n" + pad + "}"
            if s.else_block is not None:
                out += " else {\n" + self.block(s.else_block, declared, indent + 1) + "\n" + pad + "}"
            return out
        if t == "While":
            return (
                pad + f"while {self.expr(s.cond)} {{\n"
                + self.block(s.body, declared, indent + 1) + "\n" + pad + "}"
            )
        if t == "For":
            return self.for_stmt(s, declared, indent)
        if t == "Decl":
            declared.add(s.name)
            kw = "let" if s.is_const else "let mut"
            return pad + f"{kw} {s.name} = {self.expr(s.value)};"
        if t == "Assign":
            if not isinstance(s.target, N.Identifier):
                raise LogyxError("il transpiler v0 assegna solo a variabili semplici")
            name = s.target.name
            val = self.expr(s.value)
            if name in declared:
                return pad + f"{name} = {val};"
            declared.add(name)
            return pad + f"let mut {name} = {val};"
        if t == "ExprStmt":
            e = s.expr
            if isinstance(e, N.Call) and isinstance(e.callee, N.Identifier) and e.callee.name == "print":
                return pad + self.print_call(e.args)
            return pad + self.expr(e) + ";"
        raise LogyxError(f"istruzione non supportata dal transpiler v0: {t}")

    def for_stmt(self, s, declared, indent):
        pad = "    " * indent
        it = s.iterable
        if not (isinstance(it, N.Call) and isinstance(it.callee, N.Identifier)
                and it.callee.name == "range" and len(it.args) == 1):
            raise LogyxError("il transpiler v0 supporta solo 'for x in range(n)'")
        declared.add(s.var)
        n = self.expr(it.args[0])
        body = self.block(s.body, declared, indent + 1)
        return pad + f"for {s.var} in 0i64..({n}) {{\n" + body + "\n" + pad + "}"

    def print_call(self, args):
        if len(args) != 1:
            raise LogyxError("print nel transpiler v0 accetta un solo argomento")
        a = args[0]
        if isinstance(a, N.StringLit):
            fmt, fargs = self._format(a)
            return f"println!({json.dumps(fmt)}{fargs});"
        return f'println!("{{}}", {self.expr(a)});'

    # --- espressioni ---

    def _format(self, string_lit):
        fmt, fargs = "", []
        for kind, val in string_lit.parts:
            if kind == "lit":
                fmt += val.replace("{", "{{").replace("}", "}}")
            else:
                fmt += "{}"
                fargs.append(self.expr(val))
        return fmt, "".join(", " + x for x in fargs)

    def expr(self, e):
        t = type(e).__name__
        if t == "Literal":
            v = e.value
            if isinstance(v, bool):
                return "true" if v else "false"
            if v is None:
                raise LogyxError("nil non supportato dal transpiler v0")
            if isinstance(v, str):
                return json.dumps(v) + ".to_string()"
            return str(v)
        if t == "StringLit":
            fmt, fargs = self._format(e)
            return f"format!({json.dumps(fmt)}{fargs})"
        if t == "Identifier":
            return e.name
        if t == "Unary":
            return ("!" if e.op == "not" else "-") + self.expr(e.operand)
        if t == "Binary":
            return f"({self.expr(e.left)} {e.op} {self.expr(e.right)})"
        if t == "Logical":
            return f"({self.expr(e.left)} {'&&' if e.op == 'and' else '||'} {self.expr(e.right)})"
        if t == "Call":
            if isinstance(e.callee, N.Identifier) and e.callee.name == "print":
                raise LogyxError("usa print come istruzione, non dentro un'espressione")
            args = ", ".join(self.expr(a) for a in e.args)
            return f"{self.expr(e.callee)}({args})"
        raise LogyxError(f"espressione non supportata dal transpiler v0: {t}")
