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
        self.param_types = {
            f.name: dict(zip(f.params, f.param_types or [None] * len(f.params))) for f in funcs
        }
        self.func_rets = {f.name: f.ret_type for f in funcs}
        self._infer_returns(funcs)
        return "\n\n".join(self.func(f) for f in funcs) + "\n"

    def func(self, f):
        ptypes = f.param_types or [None] * len(f.params)
        params = ", ".join(f"{n}: {self.ty(t)}" for n, t in zip(f.params, ptypes))
        ret = self.func_rets.get(f.name)
        if ret is None:
            raise LogyxError(
                f"non riesco a inferire il tipo di ritorno di '{f.name}'; "
                "aggiungi l'annotazione '-> tipo'"
            )
        ret_str = "" if ret == "__void__" else f" -> {self.ty(ret)}"
        declared = set(f.params)
        body = self.block(f.body, declared, 1)
        return f"fn {f.name}({params}){ret_str} {{\n{body}\n}}"

    # --- inferenza del tipo di ritorno ---

    def _infer_returns(self, funcs):
        changed = True
        while changed:
            changed = False
            for f in funcs:
                if self.func_rets[f.name] is not None:
                    continue
                inferred = self._infer_func_ret(f)
                if inferred is not None:
                    self.func_rets[f.name] = inferred
                    changed = True

    def _infer_func_ret(self, f):
        ptypes = self.param_types[f.name]
        value_rets = [r for r in self._returns(f.body) if r.value is not None]
        if not value_rets:
            return "__void__"
        for r in value_rets:
            t = self._type_of(r.value, ptypes)
            if t is not None:
                return t
        return None

    def _returns(self, stmts):
        for s in stmts:
            t = type(s).__name__
            if t == "Return":
                yield s
            elif t == "If":
                yield from self._returns(s.then_block)
                if s.else_block:
                    yield from self._returns(s.else_block)
            elif t in ("While", "For"):
                yield from self._returns(s.body)

    def _type_of(self, e, ptypes):
        t = type(e).__name__
        if t == "StringLit":
            return "string"
        if t == "Literal":
            v = e.value
            if isinstance(v, bool):
                return "bool"
            if isinstance(v, int):
                return "int"
            if isinstance(v, float):
                return "float"
            return None
        if t == "Identifier":
            return ptypes.get(e.name)
        if t == "Unary":
            return "bool" if e.op == "not" else self._type_of(e.operand, ptypes)
        if t == "Logical":
            return "bool"
        if t == "Binary":
            if e.op in ("==", "!=", "<", "<=", ">", ">="):
                return "bool"
            lt = self._type_of(e.left, ptypes)
            rt = self._type_of(e.right, ptypes)
            if e.op == "+" and (lt == "string" or rt == "string"):
                return "string"
            if "float" in (lt, rt):
                return "float"
            if lt == "int" and rt == "int":
                return "int"
            return None
        if t == "Call" and isinstance(e.callee, N.Identifier):
            name = e.callee.name
            if name == "str":
                return "string"
            if name == "len":
                return "int"
            return self.func_rets.get(name)
        return None

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
                and it.callee.name == "range" and len(it.args) in (1, 2)):
            raise LogyxError("il transpiler v0 supporta solo 'for x in range(n)' o 'range(a, b)'")
        declared.add(s.var)
        if len(it.args) == 1:
            lo, hi = "0i64", self.expr(it.args[0])
        else:
            lo, hi = self.expr(it.args[0]), self.expr(it.args[1])
        body = self.block(s.body, declared, indent + 1)
        return pad + f"for {s.var} in ({lo})..({hi}) {{\n" + body + "\n" + pad + "}"

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

    def _stringish(self, e):
        """Euristica v0: l'espressione produce (probabilmente) una stringa."""
        if isinstance(e, N.StringLit):
            return True
        if isinstance(e, N.Literal) and isinstance(e.value, str):
            return True
        if isinstance(e, N.Binary) and e.op == "+":
            return self._stringish(e.left) or self._stringish(e.right)
        return False

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
            if isinstance(v, int):
                return f"{v}i64"
            return f"{v}f64"
        if t == "StringLit":
            if all(kind == "lit" for kind, _ in e.parts):
                text = "".join(val for _, val in e.parts)
                return json.dumps(text) + ".to_string()"
            fmt, fargs = self._format(e)
            return f"format!({json.dumps(fmt)}{fargs})"
        if t == "Identifier":
            return e.name
        if t == "Unary":
            return ("!" if e.op == "not" else "-") + self.expr(e.operand)
        if t == "Binary":
            if e.op == "+" and (self._stringish(e.left) or self._stringish(e.right)):
                return f'format!("{{}}{{}}", {self.expr(e.left)}, {self.expr(e.right)})'
            return f"({self.expr(e.left)} {e.op} {self.expr(e.right)})"
        if t == "Logical":
            return f"({self.expr(e.left)} {'&&' if e.op == 'and' else '||'} {self.expr(e.right)})"
        if t == "Call":
            if isinstance(e.callee, N.Identifier) and e.callee.name == "print":
                raise LogyxError("usa print come istruzione, non dentro un'espressione")
            args = ", ".join(self.expr(a) for a in e.args)
            return f"{self.expr(e.callee)}({args})"
        raise LogyxError(f"espressione non supportata dal transpiler v0: {t}")
