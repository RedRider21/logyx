# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later

import datetime
import hashlib
import json
import math
import time

from . import nodes as N
from .tokens import T
from .errors import LogyxError
from .client import compile_island


class _Return(Exception):
    def __init__(self, value):
        self.value = value


class _Response(Exception):
    def __init__(self, html):
        self.html = html


class _Break(Exception):
    pass


class _Continue(Exception):
    pass


class ErrValue:
    """Valore di errore recuperabile (modello Result). Porta un messaggio."""

    def __init__(self, message):
        self.message = message

    def __eq__(self, other):
        return isinstance(other, ErrValue) and other.message == self.message


class RecordValue:
    """Istanza di un record: un tipo con campi nominati."""

    def __init__(self, type_name, fields):
        self.type_name = type_name
        self.fields = fields  # dict nome_campo -> valore

    def __eq__(self, other):
        return (
            isinstance(other, RecordValue)
            and self.type_name == other.type_name
            and self.fields == other.fields
        )


class EnumValue:
    """Valore di una variante di enum (v0: senza payload)."""

    def __init__(self, type_name, variant):
        self.type_name = type_name
        self.variant = variant

    def __eq__(self, other):
        return (
            isinstance(other, EnumValue)
            and self.type_name == other.type_name
            and self.variant == other.variant
        )

    def __hash__(self):
        return hash((self.type_name, self.variant))


def html_escape(s):
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


class ExternMarker:
    """Segnaposto per una funzione `extern rust`: eseguibile solo con `build`."""

    def __init__(self, name):
        self.name = name


class LogyxFunction:
    def __init__(self, decl, closure):
        self.decl = decl
        self.closure = closure

    @property
    def arity(self):
        return len(self.decl.params)


class Environment:
    """Ambito con catena verso il genitore."""

    def __init__(self, parent=None):
        self.vars = {}
        self.consts = set()
        self.parent = parent

    def define(self, name, value, const=False):
        self.vars[name] = value
        if const:
            self.consts.add(name)

    def _holder(self, name):
        env = self
        while env is not None:
            if name in env.vars:
                return env
            env = env.parent
        return None

    def get(self, name):
        env = self._holder(name)
        if env is None:
            raise LogyxError(f"nome non definito: '{name}'")
        return env.vars[name]

    def assign(self, name, value):
        env = self._holder(name)
        if env is None:
            self.vars[name] = value
            return
        if name in env.consts:
            raise LogyxError(f"non si puo' riassegnare la costante '{name}'")
        env.vars[name] = value


def logyx_str(v):
    if v is True:
        return "true"
    if v is False:
        return "false"
    if v is None:
        return "nil"
    if isinstance(v, str):
        return v
    if isinstance(v, float):
        return str(int(v)) if v.is_integer() else repr(v)
    if isinstance(v, list):
        return "[" + ", ".join(logyx_str(x) for x in v) + "]"
    if isinstance(v, dict):
        return "{" + ", ".join(f"{logyx_str(k)}: {logyx_str(val)}" for k, val in v.items()) + "}"
    if isinstance(v, LogyxFunction):
        return f"<fn {v.decl.name}>"
    if isinstance(v, ErrValue):
        return f"<error: {v.message}>"
    if isinstance(v, RecordValue):
        inner = ", ".join(f"{k}: {logyx_str(val)}" for k, val in v.fields.items())
        return f"{v.type_name}({inner})"
    if isinstance(v, EnumValue):
        return f"{v.type_name}.{v.variant}"
    return str(v)


def _intdiv(a, b):
    """Divisione intera troncata verso lo zero (come Rust su i64)."""
    q = abs(a) // abs(b)
    return -q if (a < 0) != (b < 0) else q


def truthy(v):
    if v is None or v is False:
        return False
    if v is True:
        return True
    return bool(v)


class Interpreter:
    def __init__(self):
        self.globals = Environment()
        self.routes = {}
        self.records = {}  # nome -> lista di (nome_campo, tipo)
        self.enums = {}    # nome -> lista di varianti
        self._install_builtins()

    def _install_builtins(self):
        g = self.globals
        g.define("print", lambda *a: print(" ".join(logyx_str(x) for x in a)))
        g.define("len", lambda x: len(x))
        g.define("str", logyx_str)

        def _range(*args):
            if len(args) == 1:
                return list(range(int(args[0])))
            if len(args) == 2:
                return list(range(int(args[0]), int(args[1])))
            raise LogyxError("range accetta 1 o 2 argomenti")

        g.define("range", _range)

        def _push(lst, value):
            if not isinstance(lst, list):
                raise LogyxError("push: il primo argomento deve essere una lista")
            lst.append(value)
            return None

        g.define("push", _push)

        def _remove(lst, i):
            if not isinstance(lst, list):
                raise LogyxError("remove: il primo argomento deve essere una lista")
            if not isinstance(i, int) or isinstance(i, bool):
                raise LogyxError("remove: l'indice deve essere un intero")
            if i < 0 or i >= len(lst):
                raise LogyxError("remove: indice fuori dai limiti")
            lst.pop(i)
            return None

        g.define("remove", _remove)
        g.define("abs", lambda x: abs(x))
        g.define("min", lambda a, b: a if a <= b else b)
        g.define("max", lambda a, b: a if a >= b else b)
        g.define("upper", lambda s: s.upper())
        g.define("lower", lambda s: s.lower())
        g.define("contains", lambda lst, x: x in lst)
        g.define("trim", lambda s: s.strip())
        g.define("pow", lambda b, e: b ** e)
        g.define("sum", lambda lst: sum(lst))

        def _sort(lst):
            if not isinstance(lst, list):
                raise LogyxError("sort: il primo argomento deve essere una lista")
            lst.sort()
            return None

        g.define("sort", _sort)
        g.define("floor", lambda x: math.floor(x))
        g.define("ceil", lambda x: math.ceil(x))
        g.define("has", lambda m, k: k in m)
        # chiavi ordinate (deterministico, come in to_json): HashMap Rust non ha ordine
        g.define("keys", lambda m: sorted(m.keys()))
        g.define("values", lambda m: [m[k] for k in sorted(m.keys())])
        g.define("sqrt", lambda x: math.sqrt(x))
        g.define("index_of", lambda s, sub: s.find(sub))
        g.define("substring", lambda s, a, b: s[a:b])
        g.define("split", lambda s, sep: s.split(sep))
        g.define("join", lambda lista, sep: sep.join(lista))
        g.define("replace", lambda s, frm, to: s.replace(frm, to))

        def _map(lista, f):
            return [self.call(f, [x]) for x in lista]

        def _filter(lista, p):
            return [x for x in lista if truthy(self.call(p, [x]))]

        def _reduce(lista, init, f):
            acc = init
            for x in lista:
                acc = self.call(f, [acc, x])
            return acc

        g.define("map", _map)
        g.define("filter", _filter)
        g.define("reduce", _reduce)

        def _json_escape(s):
            out = ['"']
            for c in s:
                if c == '"':
                    out.append('\\"')
                elif c == "\\":
                    out.append("\\\\")
                elif c == "\n":
                    out.append("\\n")
                elif c == "\t":
                    out.append("\\t")
                elif c == "\r":
                    out.append("\\r")
                elif ord(c) < 0x20:
                    out.append("\\u%04x" % ord(c))
                else:
                    out.append(c)
            out.append('"')
            return "".join(out)

        def _fmt_float(x):
            if x != x or x in (float("inf"), float("-inf")):
                raise LogyxError("to_json: float non finito")
            if float(x).is_integer():
                return "%d.0" % int(x)
            return repr(x)

        def _to_json(v):
            if isinstance(v, bool):
                return "true" if v else "false"
            if isinstance(v, int):
                return str(v)
            if isinstance(v, float):
                return _fmt_float(v)
            if isinstance(v, str):
                return _json_escape(v)
            if isinstance(v, list):
                return "[" + ",".join(_to_json(x) for x in v) + "]"
            if isinstance(v, dict):
                items = []
                for k in sorted(v.keys()):
                    if not isinstance(k, str):
                        raise LogyxError("to_json di mappe: v0 supporta solo chiavi string")
                    items.append(_json_escape(k) + ":" + _to_json(v[k]))
                return "{" + ",".join(items) + "}"
            if isinstance(v, RecordValue):
                parts = [
                    '"%s":%s' % (fn, _to_json(v.fields[fn]))
                    for fn, _ in self.records[v.type_name]
                ]
                return "{" + ",".join(parts) + "}"
            raise LogyxError(
                "to_json non supporta questo tipo (v0: int, float, bool, string, record, liste)"
            )

        g.define("to_json", _to_json)
        g.define("sha256", lambda s: hashlib.sha256(s.encode("utf-8")).hexdigest())
        g.define("starts_with", lambda s, p: s.startswith(p))
        g.define("ends_with", lambda s, p: s.endswith(p))
        g.define("now", lambda: int(time.time()))
        g.define(
            "format_date",
            lambda s: datetime.datetime.fromtimestamp(s, datetime.timezone.utc).strftime(
                "%Y-%m-%d %H:%M:%S"
            ),
        )

    def load(self, items):
        """Registra funzioni e route ed esegue le istruzioni di primo livello.

        Non chiama main(): serve anche per rendere una route o avviare il server.
        Restituisce la funzione main se definita.
        """
        main = None
        for item in items:
            if isinstance(item, N.FunctionDef):
                fn = LogyxFunction(item, self.globals)
                self.globals.define(item.name, fn)
                if item.name == "main":
                    main = fn
            elif isinstance(item, N.RouteDef):
                self.routes[item.path] = item
            elif isinstance(item, N.RecordDef):
                self.records[item.name] = item.fields
            elif isinstance(item, N.EnumDef):
                self.enums[item.name] = item.variants
            elif isinstance(item, N.UseRust):
                pass  # dipendenze crate: rilevanti solo per 'build'
            elif isinstance(item, N.ExternFn):
                self.globals.define(item.name, ExternMarker(item.name))
            else:
                self.exec(item, self.globals)
        return main

    def run(self, items):
        main = self.load(items)
        if main is not None:
            self.call(main, [])

    def render_route(self, path):
        route = self.routes.get(path)
        if route is None:
            raise LogyxError(f"nessuna route definita per il percorso '{path}'")
        env = Environment(self.globals)
        try:
            self._exec_all(route.body, env)
        except _Response as r:
            return r.html
        return ""

    def eval_source(self, src, env):
        from .lexer import Lexer
        from .parser import Parser
        tokens = Lexer(src, "<template>").tokenize()
        p = Parser(tokens, "<template>")
        node = p.expression()
        if not p.at(T.EOF):
            raise LogyxError(f"espressione non valida nel template: {src!r}")
        return self.eval(node, env)

    def render_template(self, raw, env):
        out = []
        i, n = 0, len(raw)
        island_end = "@end-client"
        while i < n:
            c = raw[i]
            if c == "{":
                depth, j = 1, i + 1
                while j < n and depth > 0:
                    if raw[j] == "{":
                        depth += 1
                    elif raw[j] == "}":
                        depth -= 1
                    j += 1
                self._render_hole(raw[i + 1:j - 1], env, out)
                i = j
            elif raw.startswith("@start-client", i):
                k = raw.find(island_end, i)
                if k == -1:
                    raise LogyxError("isola client non terminata nel template")
                body_src = raw[i + len("@start-client"):k]
                out.append(compile_island(body_src))
                i = k + len(island_end)
            else:
                out.append(c)
                i += 1
        return "".join(out)

    def _render_hole(self, hole, env, out):
        s = hole.strip()
        if s.startswith("for ") or s.startswith("for\t"):
            self._render_for(s, env, out)
        elif s.startswith("if ") or s.startswith("if\t"):
            self._render_if(s, env, out)
        else:
            out.append(html_escape(logyx_str(self.eval_source(s, env))))

    def _split_block(self, s, keyword):
        """Da 'keyword <header> { <inner> } <rest>' restituisce (header, inner, rest)."""
        brace = s.index("{")
        header = s[len(keyword):brace].strip()
        depth, i = 1, brace + 1
        while i < len(s) and depth > 0:
            if s[i] == "{":
                depth += 1
            elif s[i] == "}":
                depth -= 1
            i += 1
        return header, s[brace + 1:i - 1], s[i:].strip()

    def _render_for(self, s, env, out):
        header, inner, _ = self._split_block(s, "for")
        idx = header.find(" in ")
        if idx == -1:
            raise LogyxError("ciclo 'for' nel template: manca 'in'")
        var = header[:idx].strip()
        iterable = self.eval_source(header[idx + 4:].strip(), env)
        if isinstance(iterable, dict):
            iterable = list(iterable.keys())
        for item in iterable:
            child = Environment(env)
            child.define(var, item)
            out.append(self.render_template(inner, child))

    def _render_if(self, s, env, out):
        header, inner, rest = self._split_block(s, "if")
        if truthy(self.eval_source(header, env)):
            out.append(self.render_template(inner, env))
            return
        if rest.startswith("else"):
            after = rest[4:].strip()
            if after.startswith("if"):
                self._render_hole(after, env, out)
            elif after.startswith("{"):
                depth, i = 1, 1
                while i < len(after) and depth > 0:
                    if after[i] == "{":
                        depth += 1
                    elif after[i] == "}":
                        depth -= 1
                    i += 1
                out.append(self.render_template(after[1:i - 1], env))

    # --- istruzioni ---

    def exec(self, stmt, env):
        method = getattr(self, "st_" + type(stmt).__name__, None)
        if method is None:
            raise LogyxError(f"istruzione non gestita: {type(stmt).__name__}")
        return method(stmt, env)

    def exec_block(self, stmts, env):
        self._exec_all(stmts, Environment(env))

    def _exec_all(self, stmts, env):
        for stmt in stmts:
            self.exec(stmt, env)

    def st_FunctionDef(self, s, env):
        env.define(s.name, LogyxFunction(s, env))

    def st_Decl(self, s, env):
        env.define(s.name, self.eval(s.value, env), s.is_const)

    def st_Assign(self, s, env):
        value = self.eval(s.value, env)
        target = s.target
        if isinstance(target, N.Identifier):
            env.assign(target.name, value)
        elif isinstance(target, N.Index):
            coll = self.eval(target.target, env)
            coll[self.eval(target.index, env)] = value
        else:
            raise LogyxError("bersaglio di assegnazione non valido")

    def st_ExprStmt(self, s, env):
        self.eval(s.expr, env)

    def st_If(self, s, env):
        if truthy(self.eval(s.cond, env)):
            self.exec_block(s.then_block, env)
        elif s.else_block is not None:
            self.exec_block(s.else_block, env)

    def st_While(self, s, env):
        while truthy(self.eval(s.cond, env)):
            try:
                self.exec_block(s.body, env)
            except _Break:
                break
            except _Continue:
                continue

    def st_For(self, s, env):
        iterable = self.eval(s.iterable, env)
        if isinstance(iterable, dict):
            iterable = sorted(iterable.keys())  # chiavi ordinate (deterministico, come nel nativo)
        for value in iterable:
            child = Environment(env)
            child.define(s.var, value)
            try:
                self._exec_all(s.body, child)
            except _Break:
                break
            except _Continue:
                continue

    def st_Break(self, s, env):
        raise _Break()

    def st_Continue(self, s, env):
        raise _Continue()

    def st_Return(self, s, env):
        raise _Return(self.eval(s.value, env) if s.value is not None else None)

    def st_Fail(self, s, env):
        raise _Return(ErrValue(logyx_str(self.eval(s.value, env))))

    def st_Match(self, s, env):
        subject = self.eval(s.subject, env)
        child = Environment(env)
        if isinstance(subject, ErrValue):
            child.define(s.err_var, subject.message)
            self._exec_all(s.err_block, child)
        else:
            child.define(s.ok_var, subject)
            self._exec_all(s.ok_block, child)

    def st_MatchValue(self, s, env):
        subject = self.eval(s.subject, env)
        for pat, block in s.cases:
            if subject == self.eval(pat, env):
                self.exec_block(block, env)
                return
        if s.else_block is not None:
            self.exec_block(s.else_block, env)

    def st_Render(self, s, env):
        raise _Response(self.render_template(s.raw, env))

    # --- espressioni ---

    def eval(self, node, env):
        method = getattr(self, "ex_" + type(node).__name__, None)
        if method is None:
            raise LogyxError(f"espressione non gestita: {type(node).__name__}")
        return method(node, env)

    def ex_Literal(self, n, env):
        return n.value

    def ex_StringLit(self, n, env):
        out = []
        for kind, val in n.parts:
            out.append(val if kind == "lit" else logyx_str(self.eval(val, env)))
        return "".join(out)

    def ex_Identifier(self, n, env):
        return env.get(n.name)

    def ex_ListLit(self, n, env):
        return [self.eval(e, env) for e in n.elements]

    def ex_MapLit(self, n, env):
        return {self.eval(k, env): self.eval(v, env) for k, v in n.pairs}

    def ex_Unary(self, n, env):
        v = self.eval(n.operand, env)
        if n.op == "not":
            return not truthy(v)
        if n.op == "-":
            return -v
        raise LogyxError(f"operatore unario sconosciuto: {n.op}")

    def ex_Logical(self, n, env):
        left = self.eval(n.left, env)
        if n.op == "and":
            return self.eval(n.right, env) if truthy(left) else left
        return left if truthy(left) else self.eval(n.right, env)

    def ex_Binary(self, n, env):
        a = self.eval(n.left, env)
        b = self.eval(n.right, env)
        op = n.op
        if op == "+":
            if isinstance(a, str) or isinstance(b, str):
                return logyx_str(a) + logyx_str(b)
            return a + b
        if op == "-":
            return a - b
        if op == "*":
            return a * b
        if op == "/":
            if b == 0:
                raise LogyxError("divisione per zero")
            if isinstance(a, int) and isinstance(b, int):
                return _intdiv(a, b)
            return a / b
        if op == "%":
            if b == 0:
                raise LogyxError("modulo per zero")
            if isinstance(a, int) and isinstance(b, int):
                return a - _intdiv(a, b) * b
            return a % b
        if op == "==":
            return a == b
        if op == "!=":
            return a != b
        if op == "<":
            return a < b
        if op == "<=":
            return a <= b
        if op == ">":
            return a > b
        if op == ">=":
            return a >= b
        raise LogyxError(f"operatore sconosciuto: {op}")

    def ex_Try(self, n, env):
        v = self.eval(n.operand, env)
        if isinstance(v, ErrValue):
            raise _Return(v)  # propaga l'errore alla funzione chiamante
        return v

    def ex_Index(self, n, env):
        coll = self.eval(n.target, env)
        idx = self.eval(n.index, env)
        try:
            return coll[idx]
        except Exception:
            raise LogyxError("indice non valido")

    def ex_Call(self, n, env):
        if isinstance(n.callee, N.Identifier):
            cname = n.callee.name
            if cname in self.records:
                return self._make_record(cname, [self.eval(a, env) for a in n.args])
            if cname == "from_json":
                return self._from_json(n, env)
        callee = self.eval(n.callee, env)
        args = [self.eval(a, env) for a in n.args]
        return self.call(callee, args)

    def _from_json(self, n, env):
        if len(n.args) != 2 or not isinstance(n.args[1], N.Identifier):
            raise LogyxError("from_json richiede: from_json(testo, NomeRecord)")
        rec = n.args[1].name
        if rec not in self.records:
            raise LogyxError(f"from_json: '{rec}' non è un record")
        testo = self.eval(n.args[0], env)
        try:
            data = json.loads(testo)
        except Exception:
            raise LogyxError("from_json: JSON non valido")
        fields = {}
        for fname, _ in self.records[rec]:
            if fname not in data:
                raise LogyxError(f"from_json: campo mancante '{fname}'")
            fields[fname] = data[fname]
        return RecordValue(rec, fields)

    def _make_record(self, name, args):
        fields = self.records[name]
        if len(args) != len(fields):
            raise LogyxError(
                f"il record '{name}' ha {len(fields)} campi, forniti {len(args)}"
            )
        return RecordValue(name, {fname: val for (fname, _), val in zip(fields, args)})

    def ex_Field(self, n, env):
        # variante di enum: Nome.Variante (Nome è un enum, non una variabile)
        if isinstance(n.target, N.Identifier) and n.target.name in self.enums:
            if n.name in self.enums[n.target.name]:
                return EnumValue(n.target.name, n.name)
            raise LogyxError(
                f"l'enum '{n.target.name}' non ha la variante '{n.name}'"
            )
        target = self.eval(n.target, env)
        if isinstance(target, RecordValue) and n.name in target.fields:
            return target.fields[n.name]
        raise LogyxError(f"campo '{n.name}' non trovato")

    def call(self, callee, args):
        if isinstance(callee, ExternMarker):
            raise LogyxError(
                f"'{callee.name}' è una funzione extern (Rust): disponibile solo con 'build', "
                "non nell'interprete"
            )
        if isinstance(callee, LogyxFunction):
            if len(args) != callee.arity:
                raise LogyxError(
                    f"la funzione '{callee.decl.name}' attende {callee.arity} "
                    f"argomenti, ricevuti {len(args)}"
                )
            env = Environment(callee.closure)
            for name, value in zip(callee.decl.params, args):
                env.define(name, value)
            try:
                self._exec_all(callee.decl.body, env)
            except _Return as r:
                return r.value
            return None
        if callable(callee):
            return callee(*args)
        raise LogyxError("valore non chiamabile")
