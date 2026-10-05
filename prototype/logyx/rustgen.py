# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Transpiler v0: da un sottoinsieme tipizzato di Logyx a codice Rust.

Supporta: funzioni con tipi espliciti (int/float/bool/string), aritmetica,
confronti e logica, if/else, while, for su range(n), return, print, ricorsione.
Non supporta ancora liste/mappe, route/render o codice dinamico: in quei casi
solleva un errore chiaro. Serve a dimostrare il percorso di compilazione nativa.
"""

from . import nodes as N
from .errors import LogyxError

_TYPES = {"int": "i64", "float": "f64", "bool": "bool", "string": "String"}

_JSON_HELPER = r'''fn __json_str(s: &str) -> String {
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
}'''


def _rust_str(s):
    """Letterale di stringa Rust valido (UTF-8 diretto, escape alla Rust).

    Nota: non si usa json.dumps perché produce escape \\uXXXX, che Rust rifiuta
    (vuole \\u{XXXX}); i caratteri non-ASCII vanno invece lasciati così come sono.
    """
    out = ['"']
    for ch in s:
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\t":
            out.append("\\t")
        elif ch == "\r":
            out.append("\\r")
        elif ord(ch) < 0x20:
            out.append(f"\\u{{{ord(ch):x}}}")
        else:
            out.append(ch)
    out.append('"')
    return "".join(out)


class RustTranspiler:
    def transpile(self, items):
        funcs = [i for i in items if isinstance(i, N.FunctionDef)]
        records = [i for i in items if isinstance(i, N.RecordDef)]
        externs = [i for i in items if isinstance(i, N.ExternFn)]
        uses = [i for i in items if isinstance(i, N.UseRust)]
        allowed = (N.FunctionDef, N.RecordDef, N.ExternFn, N.UseRust)
        others = [i for i in items if not isinstance(i, allowed)]
        if others:
            raise LogyxError(
                "il transpiler Rust v0 supporta solo funzioni, record, use/extern rust "
                "(niente route/render o codice a primo livello)"
            )
        if not any(f.name == "main" for f in funcs):
            raise LogyxError("manca 'fn main()': serve un punto d'ingresso")
        self.records = {r.name: r.fields for r in records}
        self.param_types = {
            f.name: dict(zip(f.params, f.param_types or [None] * len(f.params))) for f in funcs
        }
        self.func_rets = {}
        self.fallible = {}
        for f in funcs:
            rt, fal = f.ret_type, False
            if rt and rt.endswith("|error"):
                rt, fal = rt[: -len("|error")], True
            if self._body_has_fail_or_try(f.body):
                fal = True
            self.func_rets[f.name] = rt or None
            self.fallible[f.name] = fal
        # funzioni extern: firma dichiarata (tipi noti, non fallibili)
        for ex in externs:
            self.param_types[ex.name] = dict(zip(ex.params, ex.param_types))
            self.func_rets[ex.name] = ex.ret_type
            self.fallible[ex.name] = False
        self._tmp = 0
        self.uses_json = False
        self.uses_serde = False
        self.deps = {}  # crate -> spec TOML (dopo '='), es. '"0.10"' o '{ version = "1", ... }'
        for u in uses:
            self.deps[u.crate] = f'"{u.version}"'
        self._infer(funcs)
        extern_defs = []
        for ex in externs:
            params = ", ".join(f"{n}: {self.ty(t)}" for n, t in zip(ex.params, ex.param_types))
            extern_defs.append(f"fn {ex.name}({params}) -> {self.ty(ex.ret_type)} {{ {ex.body} }}")
        func_defs = [self.func(f) for f in funcs]  # può impostare uses_json / uses_serde / deps
        derive = "Clone, PartialEq, Serialize, Deserialize" if self.uses_serde else "Clone, PartialEq"
        struct_defs = []
        for name in sorted(self.records):
            fields = ", ".join(f"{fn}: {self.ty(ft)}" for fn, ft in self.records[name])
            struct_defs.append(f"#[derive({derive})]\nstruct {name} {{ {fields} }}")
        pieces = []
        if self.uses_serde:
            pieces.append("use serde::{Serialize, Deserialize};")
        if self.uses_json:
            pieces.append(_JSON_HELPER)
        pieces += struct_defs + extern_defs + func_defs
        return "\n\n".join(pieces) + "\n"

    # --- rilevazione di fallibilità (presenza di `fail` o `?`) ---

    def _body_has_fail_or_try(self, stmts):
        for s in stmts:
            t = type(s).__name__
            if t == "Fail":
                return True
            if t == "Return":
                if s.value is not None and self._expr_has_try(s.value):
                    return True
            elif t in ("Decl", "Assign"):
                if self._expr_has_try(s.value):
                    return True
            elif t == "ExprStmt":
                if self._expr_has_try(s.expr):
                    return True
            elif t == "If":
                if self._body_has_fail_or_try(s.then_block):
                    return True
                if s.else_block and self._body_has_fail_or_try(s.else_block):
                    return True
            elif t in ("While", "For"):
                if self._body_has_fail_or_try(s.body):
                    return True
            elif t == "Match":
                if self._expr_has_try(s.subject) or self._body_has_fail_or_try(s.ok_block) \
                        or self._body_has_fail_or_try(s.err_block):
                    return True
            elif t == "MatchValue":
                if self._expr_has_try(s.subject) \
                        or any(self._body_has_fail_or_try(blk) for _, blk in s.cases) \
                        or (s.else_block and self._body_has_fail_or_try(s.else_block)):
                    return True
        return False

    def _expr_has_try(self, e):
        t = type(e).__name__
        if t == "Try":
            return True
        if t in ("Binary", "Logical"):
            return self._expr_has_try(e.left) or self._expr_has_try(e.right)
        if t == "Unary":
            return self._expr_has_try(e.operand)
        if t == "Call":
            return any(self._expr_has_try(a) for a in e.args)
        if t == "Index":
            return self._expr_has_try(e.target) or self._expr_has_try(e.index)
        if t == "Field":
            return self._expr_has_try(e.target)
        if t == "StringLit":
            return any(self._expr_has_try(v) for k, v in e.parts if k != "lit")
        if t == "ListLit":
            return any(self._expr_has_try(x) for x in e.elements)
        if t == "MapLit":
            return any(self._expr_has_try(k) or self._expr_has_try(v) for k, v in e.pairs)
        return False

    def func(self, f):
        pt = self.param_types[f.name]
        missing = [n for n in f.params if pt.get(n) is None]
        if missing:
            raise LogyxError(
                f"non riesco a inferire il tipo del parametro '{missing[0]}' di '{f.name}'; "
                f"aggiungi l'annotazione '{missing[0]}: tipo'"
            )
        params = ", ".join(f"{n}: {self.ty(pt[n])}" for n in f.params)
        ret = self.func_rets.get(f.name)
        if ret is None:
            raise LogyxError(
                f"non riesco a inferire il tipo di ritorno di '{f.name}'; "
                "aggiungi l'annotazione '-> tipo'"
            )
        self.cur_fallible = self.fallible.get(f.name, False)
        if self.cur_fallible:
            inner = "()" if ret == "__void__" else self.ty(ret)
            ret_str = f" -> Result<{inner}, String>"
        else:
            ret_str = "" if ret == "__void__" else f" -> {self.ty(ret)}"
        declared = set(f.params)
        self.kinds = {}  # nome -> "list" | "map" (categoria delle variabili locali)
        self.cur_types = {n: t for n, t in pt.items() if t}  # tipi noti (param + locali)
        body = self.block(f.body, declared, 1)
        return f"fn {f.name}({params}){ret_str} {{\n{body}\n}}"

    @staticmethod
    def _value_kind(v):
        t = type(v).__name__
        if t == "ListLit":
            return "list"
        if t == "MapLit":
            return "map"
        return None

    # --- inferenza (punto fisso su parametri e tipi di ritorno) ---

    def _infer(self, funcs):
        changed = True
        while changed:
            changed = False
            if self._infer_params_once(funcs):
                changed = True
            if self._infer_returns_once(funcs):
                changed = True

    def _infer_returns_once(self, funcs):
        changed = False
        for f in funcs:
            if self.func_rets[f.name] is not None:
                continue
            inferred = self._infer_func_ret(f)
            if inferred is not None:
                self.func_rets[f.name] = inferred
                changed = True
        return changed

    # --- inferenza dei tipi dei parametri dall'uso nel corpo ---

    def _infer_params_once(self, funcs):
        changed = False
        for f in funcs:
            pt = self.param_types[f.name]
            ev = {p: set() for p in f.params if pt.get(p) is None}
            if not ev:
                continue
            self._scan_stmts(f.body, pt, ev, f.name)
            for p, e in ev.items():
                t = self._resolve_ev(e)
                if t is not None:
                    pt[p] = t
                    changed = True
        return changed

    @staticmethod
    def _resolve_ev(e):
        # priorita': string > float > int/num > bool
        if "string" in e:
            return "string"
        if "float" in e:
            return "float"
        if "int" in e or "num" in e:
            return "int"
        if "bool" in e:
            return "bool"
        return None

    def _scan_stmts(self, stmts, pt, ev, fname):
        for s in stmts:
            self._scan_stmt(s, pt, ev, fname)

    def _scan_stmt(self, s, pt, ev, fname):
        t = type(s).__name__
        if t == "If":
            self._cond_bool(s.cond, ev)
            self._scan_expr(s.cond, pt, ev)
            self._scan_stmts(s.then_block, pt, ev, fname)
            if s.else_block:
                self._scan_stmts(s.else_block, pt, ev, fname)
        elif t == "While":
            self._cond_bool(s.cond, ev)
            self._scan_expr(s.cond, pt, ev)
            self._scan_stmts(s.body, pt, ev, fname)
        elif t == "For":
            self._scan_expr(s.iterable, pt, ev)
            self._scan_stmts(s.body, pt, ev, fname)
        elif t == "Return":
            if s.value is not None:
                rt = self.func_rets.get(fname)
                if isinstance(s.value, N.Identifier) and s.value.name in ev and rt in _TYPES:
                    ev[s.value.name].add(rt)
                self._scan_expr(s.value, pt, ev)
        elif t in ("Decl", "Assign"):
            self._scan_expr(s.value, pt, ev)
        elif t == "ExprStmt":
            self._scan_expr(s.expr, pt, ev)
        elif t == "Fail":
            self._scan_expr(s.value, pt, ev)
        elif t == "Match":
            self._scan_expr(s.subject, pt, ev)
            self._scan_stmts(s.ok_block, pt, ev, fname)
            self._scan_stmts(s.err_block, pt, ev, fname)
        elif t == "MatchValue":
            for pat, blk in s.cases:
                self._compare_ev(s.subject, pat, pt, ev)
                self._compare_ev(pat, s.subject, pt, ev)
                self._scan_expr(pat, pt, ev)
                self._scan_stmts(blk, pt, ev, fname)
            self._scan_expr(s.subject, pt, ev)
            if s.else_block:
                self._scan_stmts(s.else_block, pt, ev, fname)

    @staticmethod
    def _cond_bool(cond, ev):
        if isinstance(cond, N.Identifier) and cond.name in ev:
            ev[cond.name].add("bool")

    def _scan_expr(self, e, pt, ev):
        t = type(e).__name__
        if t == "Binary":
            self._binary_ev(e, pt, ev)
            self._scan_expr(e.left, pt, ev)
            self._scan_expr(e.right, pt, ev)
        elif t == "Logical":
            for side in (e.left, e.right):
                if isinstance(side, N.Identifier) and side.name in ev:
                    ev[side.name].add("bool")
            self._scan_expr(e.left, pt, ev)
            self._scan_expr(e.right, pt, ev)
        elif t == "Unary":
            if isinstance(e.operand, N.Identifier) and e.operand.name in ev:
                ev[e.operand.name].add("bool" if e.op == "not" else "num")
            self._scan_expr(e.operand, pt, ev)
        elif t == "Try":
            self._scan_expr(e.operand, pt, ev)
        elif t == "Call":
            self._call_ev(e, pt, ev)
            for a in e.args:
                self._scan_expr(a, pt, ev)
        elif t == "StringLit":
            for kind, val in e.parts:
                if kind != "lit":
                    self._scan_expr(val, pt, ev)
        elif t == "Index":
            if isinstance(e.index, N.Identifier) and e.index.name in ev:
                ev[e.index.name].add("int")
            self._scan_expr(e.target, pt, ev)
            self._scan_expr(e.index, pt, ev)
        elif t == "Field":
            self._scan_expr(e.target, pt, ev)

    def _binary_ev(self, e, pt, ev):
        op, L, R = e.op, e.left, e.right
        if op in ("==", "!=", "<", "<=", ">", ">="):
            self._compare_ev(L, R, pt, ev)
            self._compare_ev(R, L, pt, ev)
        elif op in ("-", "*", "/", "%"):
            self._arith_ev(L, R, pt, ev)
            self._arith_ev(R, L, pt, ev)
        elif op == "+":
            lt, rt = self._type_of(L, pt), self._type_of(R, pt)
            if "string" in (lt, rt) or self._stringish(L) or self._stringish(R):
                for side in (L, R):
                    if isinstance(side, N.Identifier) and side.name in ev:
                        ev[side.name].add("string")
            else:
                self._arith_ev(L, R, pt, ev)
                self._arith_ev(R, L, pt, ev)

    def _compare_ev(self, x, other, pt, ev):
        if isinstance(x, N.Identifier) and x.name in ev:
            ot = self._type_of(other, pt)
            ev[x.name].add(ot if ot in _TYPES else "num")

    def _arith_ev(self, x, other, pt, ev):
        if isinstance(x, N.Identifier) and x.name in ev:
            ot = self._type_of(other, pt)
            ev[x.name].add(ot if ot in ("int", "float") else "num")

    def _call_ev(self, e, pt, ev):
        if not isinstance(e.callee, N.Identifier):
            return
        name = e.callee.name
        if name == "range":
            for a in e.args:
                if isinstance(a, N.Identifier) and a.name in ev:
                    ev[a.name].add("int")
            return
        if name in ("abs", "min", "max", "pow", "floor", "ceil", "sqrt"):
            for a in e.args:
                if isinstance(a, N.Identifier) and a.name in ev:
                    ev[a.name].add("num")
            return
        if name in ("upper", "lower", "trim", "index_of", "split", "replace",
                    "starts_with", "ends_with"):
            for a in e.args:
                if isinstance(a, N.Identifier) and a.name in ev:
                    ev[a.name].add("string")
            return
        if name == "join":
            if len(e.args) == 2 and isinstance(e.args[1], N.Identifier) and e.args[1].name in ev:
                ev[e.args[1].name].add("string")
            return
        if name == "substring":
            if e.args and isinstance(e.args[0], N.Identifier) and e.args[0].name in ev:
                ev[e.args[0].name].add("string")
            for a in e.args[1:3]:
                if isinstance(a, N.Identifier) and a.name in ev:
                    ev[a.name].add("int")
            return
        if name == "remove":
            if len(e.args) == 2 and isinstance(e.args[1], N.Identifier) and e.args[1].name in ev:
                ev[e.args[1].name].add("int")
            return
        if name in self.param_types:
            pnames = list(self.param_types[name].keys())
            for i, a in enumerate(e.args):
                if i >= len(pnames):
                    break
                target_t = self.param_types[name][pnames[i]]
                if target_t in _TYPES and isinstance(a, N.Identifier) and a.name in ev:
                    ev[a.name].add(target_t)

    def _infer_func_ret(self, f):
        types = self._local_types(f, self.param_types[f.name])
        value_rets = [r for r in self._returns(f.body) if r.value is not None]
        if not value_rets:
            return "__void__"
        for r in value_rets:
            t = self._type_of(r.value, types)
            if t is not None:
                return t
        return None

    def _local_types(self, f, ptypes):
        """Tipi dei parametri + tipi dedotti delle variabili locali (per l'inferenza)."""
        types = dict(ptypes)
        self._collect_local_types(f.body, types)
        return types

    def _collect_local_types(self, stmts, types):
        for s in stmts:
            t = type(s).__name__
            if t == "Decl":
                vt = self._type_of(s.value, types)
                if vt:
                    types[s.name] = vt
            elif t == "Assign":
                if isinstance(s.target, N.Identifier):
                    vt = self._type_of(s.value, types)
                    if vt:
                        types[s.target.name] = vt
            elif t == "If":
                self._collect_local_types(s.then_block, types)
                if s.else_block:
                    self._collect_local_types(s.else_block, types)
            elif t in ("While", "For"):
                self._collect_local_types(s.body, types)
            elif t == "Match":
                self._collect_local_types(s.ok_block, types)
                self._collect_local_types(s.err_block, types)
            elif t == "MatchValue":
                for _, blk in s.cases:
                    self._collect_local_types(blk, types)
                if s.else_block:
                    self._collect_local_types(s.else_block, types)

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
            elif t == "Match":
                yield from self._returns(s.ok_block)
                yield from self._returns(s.err_block)
            elif t == "MatchValue":
                for _, blk in s.cases:
                    yield from self._returns(blk)
                if s.else_block:
                    yield from self._returns(s.else_block)

    def _type_of(self, e, ptypes):
        t = type(e).__name__
        if t == "Try":
            return self._type_of(e.operand, ptypes)
        if t == "Field":
            tt = self._type_of(e.target, ptypes)
            for fn, ft in getattr(self, "records", {}).get(tt, []):
                if fn == e.name:
                    return ft
            return None
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
            if name in ("abs", "pow") and e.args:
                return self._type_of(e.args[0], ptypes)
            if name in ("min", "max"):
                for a in e.args:
                    tt = self._type_of(a, ptypes)
                    if tt is not None:
                        return tt
                return None
            if name in ("upper", "lower", "trim", "substring", "join", "replace",
                        "to_json", "sha256", "format_date"):
                return "string"
            if name == "now":
                return "int"
            if name in ("contains", "has", "starts_with", "ends_with"):
                return "bool"
            if name == "index_of":
                return "int"
            if name == "sum":
                return "int"
            if name in ("floor", "ceil"):
                return "int"
            if name == "sqrt":
                return "float"
            if name == "reduce" and len(e.args) >= 2:
                return self._type_of(e.args[1], ptypes)
            if name in getattr(self, "records", {}):
                return name
            if name == "from_json" and len(e.args) == 2 and isinstance(e.args[1], N.Identifier):
                return e.args[1].name
            return self.func_rets.get(name)
        return None

    def ty(self, t):
        if t is None:
            raise LogyxError("il transpiler v0 richiede tipi espliciti su parametri e tipo di ritorno")
        if t in _TYPES:
            return _TYPES[t]
        if t in getattr(self, "records", {}):
            return t
        raise LogyxError(f"tipo non supportato dal transpiler v0: '{t}'")

    # --- istruzioni ---

    def block(self, stmts, declared, indent):
        return "\n".join(self.stmt(s, declared, indent) for s in stmts)

    def stmt(self, s, declared, indent):
        pad = "    " * indent
        t = type(s).__name__
        if t == "Return":
            if self.cur_fallible:
                inner = "()" if s.value is None else self.expr(s.value)
                return pad + f"return Ok({inner});"
            return pad + ("return;" if s.value is None else f"return {self.expr(s.value)};")
        if t == "Fail":
            return pad + f"return Err({self.expr(s.value)});"
        if t == "Match":
            subj = self.expr(s.subject)
            ok_decl, err_decl = set(declared), set(declared)
            ok_decl.add(s.ok_var)
            err_decl.add(s.err_var)
            okb = self.block(s.ok_block, ok_decl, indent + 2)
            errb = self.block(s.err_block, err_decl, indent + 2)
            arm = "    " * (indent + 1)
            return (
                pad + f"match {subj} {{\n"
                + arm + f"Ok({s.ok_var}) => {{\n" + okb + "\n" + arm + "}\n"
                + arm + f"Err({s.err_var}) => {{\n" + errb + "\n" + arm + "}\n"
                + pad + "}"
            )
        if t == "MatchValue":
            subj = self.expr(s.subject)
            out = ""
            for i, (pat, blk) in enumerate(s.cases):
                body = self.block(blk, declared, indent + 1)
                cond = f"({subj} == {self.expr(pat)})"
                head = "if" if i == 0 else " else if"
                lead = pad if i == 0 else ""
                out += lead + f"{head} {cond} {{\n" + body + "\n" + pad + "}"
            if s.else_block is not None:
                body = self.block(s.else_block, declared, indent + 1)
                if not s.cases:
                    out = pad + "{\n" + body + "\n" + pad + "}"
                else:
                    out += " else {\n" + body + "\n" + pad + "}"
            return out or (pad + "{}")
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
        if t == "Break":
            return pad + "break;"
        if t == "Continue":
            return pad + "continue;"
        if t == "Decl":
            declared.add(s.name)
            k = self._value_kind(s.value)
            if k:
                self.kinds[s.name] = k
            vt = self._type_of(s.value, self.cur_types)
            if vt:
                self.cur_types[s.name] = vt
            kw = "let" if s.is_const else "let mut"
            return pad + f"{kw} {s.name} = {self.expr(s.value)};"
        if t == "Assign":
            if not isinstance(s.target, N.Identifier):
                raise LogyxError("il transpiler v0 assegna solo a variabili semplici")
            name = s.target.name
            k = self._value_kind(s.value)
            if k:
                self.kinds[name] = k
            vt = self._type_of(s.value, self.cur_types)
            if vt:
                self.cur_types[name] = vt
            val = self.expr(s.value)
            if name in declared:
                return pad + f"{name} = {val};"
            declared.add(name)
            return pad + f"let mut {name} = {val};"
        if t == "ExprStmt":
            e = s.expr
            if isinstance(e, N.Call) and isinstance(e.callee, N.Identifier):
                if e.callee.name == "print":
                    return pad + self.print_call(e.args)
                if e.callee.name == "push":
                    return pad + self.push_call(e.args)
                if e.callee.name == "remove":
                    return pad + self.remove_call(e.args)
                if e.callee.name == "sort":
                    return pad + self.sort_call(e.args)
            return pad + self.expr(e) + ";"
        raise LogyxError(f"istruzione non supportata dal transpiler v0: {t}")

    def for_stmt(self, s, declared, indent):
        pad = "    " * indent
        it = s.iterable
        is_range = (isinstance(it, N.Call) and isinstance(it.callee, N.Identifier)
                    and it.callee.name == "range" and len(it.args) in (1, 2))
        declared.add(s.var)
        if is_range:
            if len(it.args) == 1:
                lo, hi = "0i64", self.expr(it.args[0])
            else:
                lo, hi = self.expr(it.args[0]), self.expr(it.args[1])
            body = self.block(s.body, declared, indent + 1)
            return pad + f"for {s.var} in ({lo})..({hi}) {{\n" + body + "\n" + pad + "}"
        # iterazione su lista: per valore; cloned() vale sia per gli scalari sia per String
        body = self.block(s.body, declared, indent + 1)
        return pad + f"for {s.var} in ({self.expr(it)}).iter().cloned() {{\n" + body + "\n" + pad + "}"

    def push_call(self, args):
        if len(args) != 2:
            raise LogyxError("push accetta due argomenti: push(lista, valore)")
        if not isinstance(args[0], N.Identifier):
            raise LogyxError("push nel transpiler v0 richiede una variabile lista come primo argomento")
        return f"{args[0].name}.push({self.expr(args[1])});"

    def remove_call(self, args):
        if len(args) != 2:
            raise LogyxError("remove accetta due argomenti: remove(lista, indice)")
        if not isinstance(args[0], N.Identifier):
            raise LogyxError("remove nel transpiler v0 richiede una variabile lista come primo argomento")
        return f"{args[0].name}.remove(({self.expr(args[1])}) as usize);"

    def sort_call(self, args):
        if len(args) != 1:
            raise LogyxError("sort accetta un solo argomento: sort(lista)")
        if not isinstance(args[0], N.Identifier):
            raise LogyxError("sort nel transpiler v0 richiede una variabile lista come argomento")
        return f"{args[0].name}.sort();"

    def _json_value(self, rust_expr, type_str):
        if type_str in ("int", "bool"):
            return f'format!("{{}}", ({rust_expr}))'
        if type_str == "string":
            return f"__json_str(&({rust_expr}))"
        if type_str in getattr(self, "records", {}):
            parts, args = [], []
            for fn, ft in self.records[type_str]:
                parts.append('\\"' + fn + '\\":{}')
                args.append(self._json_value(f"({rust_expr}).{fn}", ft))
            fmt = "{{" + ",".join(parts) + "}}"
            return 'format!("' + fmt + '", ' + ", ".join(args) + ")"
        raise LogyxError(
            f"to_json non supporta il tipo '{type_str}' (v0: int, bool, string, record)"
        )

    def _make_record_expr(self, name, args):
        fields = self.records[name]
        if len(args) != len(fields):
            raise LogyxError(f"il record '{name}' ha {len(fields)} campi, forniti {len(args)}")
        parts = ", ".join(f"{fn}: {self._arg(a)}" for (fn, _), a in zip(fields, args))
        return f"{name} {{ {parts} }}"

    def _arg(self, a):
        # Una variabile passata a una funzione la "muove"; cloniamo per riusabilità.
        # Per i tipi Copy il clone è gratuito dopo l'ottimizzazione.
        if isinstance(a, N.Identifier):
            return self.expr(a) + ".clone()"
        return self.expr(a)

    def _fn_name(self, args, n, idx, usage):
        base = usage.split("(")[0]
        if len(args) != n:
            raise LogyxError(f"{base} accetta {n} argomenti: {usage}")
        f = args[idx]
        if not isinstance(f, N.Identifier):
            raise LogyxError(f"{base}: l'argomento funzione deve essere il nome di una funzione definita con 'fn'")
        return f.name

    def print_call(self, args):
        if len(args) != 1:
            raise LogyxError("print nel transpiler v0 accetta un solo argomento")
        a = args[0]
        if isinstance(a, N.StringLit):
            fmt, fargs = self._format(a)
            return f"println!({_rust_str(fmt)}{fargs});"
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

    def _is_string_expr(self, e):
        """Come `_stringish`, ma usa anche i tipi noti (param + locali): così
        `r = r + testo` fra due `String` genera `format!`, non `r + testo`."""
        if self._stringish(e):
            return True
        return self._type_of(e, getattr(self, "cur_types", {})) == "string"

    def expr(self, e):
        t = type(e).__name__
        if t == "Literal":
            v = e.value
            if isinstance(v, bool):
                return "true" if v else "false"
            if v is None:
                raise LogyxError("nil non supportato dal transpiler v0")
            if isinstance(v, str):
                return _rust_str(v) + ".to_string()"
            if isinstance(v, int):
                return f"{v}i64"
            return f"{v}f64"
        if t == "StringLit":
            if all(kind == "lit" for kind, _ in e.parts):
                text = "".join(val for _, val in e.parts)
                return _rust_str(text) + ".to_string()"
            fmt, fargs = self._format(e)
            return f"format!({_rust_str(fmt)}{fargs})"
        if t == "Identifier":
            return e.name
        if t == "Unary":
            return ("!" if e.op == "not" else "-") + self.expr(e.operand)
        if t == "Binary":
            if e.op == "+" and (self._is_string_expr(e.left) or self._is_string_expr(e.right)):
                return f'format!("{{}}{{}}", {self.expr(e.left)}, {self.expr(e.right)})'
            return f"({self.expr(e.left)} {e.op} {self.expr(e.right)})"
        if t == "Logical":
            return f"({self.expr(e.left)} {'&&' if e.op == 'and' else '||'} {self.expr(e.right)})"
        if t == "ListLit":
            if not e.elements:
                raise LogyxError(
                    "il transpiler v0 non deduce il tipo di una lista vuota; "
                    "usa una lista con almeno un elemento"
                )
            return "vec![" + ", ".join(self.expr(x) for x in e.elements) + "]"
        if t == "MapLit":
            if not e.pairs:
                raise LogyxError(
                    "il transpiler v0 non deduce il tipo di una mappa vuota; "
                    "usa una mappa con almeno una coppia"
                )
            name = f"__m{self._tmp}"
            self._tmp += 1
            inserts = " ".join(
                f"{name}.insert({self.expr(k)}, {self.expr(v)});" for k, v in e.pairs
            )
            return f"{{ let mut {name} = std::collections::HashMap::new(); {inserts} {name} }}"
        if t == "Try":
            return f"({self.expr(e.operand)})?"
        if t == "Index":
            if isinstance(e.target, N.Identifier) and getattr(self, "kinds", {}).get(e.target.name) == "map":
                return f"{self.expr(e.target)}.get(&({self.expr(e.index)})).unwrap().clone()"
            return f"{self.expr(e.target)}[({self.expr(e.index)}) as usize].clone()"
        if t == "Field":
            return f"{self.expr(e.target)}.{e.name}.clone()"
        if t == "Call":
            if isinstance(e.callee, N.Identifier) and e.callee.name in getattr(self, "records", {}):
                return self._make_record_expr(e.callee.name, e.args)
            if isinstance(e.callee, N.Identifier):
                nm = e.callee.name
                if nm == "print":
                    raise LogyxError("usa print come istruzione, non dentro un'espressione")
                if nm == "push":
                    raise LogyxError("usa push come istruzione, non dentro un'espressione")
                if nm == "remove":
                    raise LogyxError("usa remove come istruzione, non dentro un'espressione")
                if nm == "len":
                    if len(e.args) != 1:
                        raise LogyxError("len accetta un solo argomento")
                    return f"(({self.expr(e.args[0])}).len() as i64)"
                if nm == "str":
                    if len(e.args) != 1:
                        raise LogyxError("str accetta un solo argomento")
                    return f'format!("{{}}", {self.expr(e.args[0])})'
                if nm == "abs":
                    if len(e.args) != 1:
                        raise LogyxError("abs accetta un solo argomento")
                    return f"({self.expr(e.args[0])}).abs()"
                if nm in ("min", "max"):
                    if len(e.args) != 2:
                        raise LogyxError(f"{nm} accetta due argomenti")
                    return f"({self.expr(e.args[0])}).{nm}({self.expr(e.args[1])})"
                if nm in ("upper", "lower"):
                    if len(e.args) != 1:
                        raise LogyxError(f"{nm} accetta un solo argomento")
                    method = "to_uppercase" if nm == "upper" else "to_lowercase"
                    return f"({self.expr(e.args[0])}).{method}()"
                if nm == "contains":
                    if len(e.args) != 2:
                        raise LogyxError("contains accetta due argomenti: contains(lista, valore)")
                    return f"({self.expr(e.args[0])}).contains(&({self.expr(e.args[1])}))"
                if nm == "has":
                    if len(e.args) != 2:
                        raise LogyxError("has accetta due argomenti: has(mappa, chiave)")
                    return f"({self.expr(e.args[0])}).contains_key(&({self.expr(e.args[1])}))"
                if nm == "trim":
                    if len(e.args) != 1:
                        raise LogyxError("trim accetta un solo argomento")
                    return f"({self.expr(e.args[0])}).trim().to_string()"
                if nm == "pow":
                    if len(e.args) != 2:
                        raise LogyxError("pow accetta due argomenti: pow(base, esponente)")
                    return f"({self.expr(e.args[0])}).pow(({self.expr(e.args[1])}) as u32)"
                if nm == "sum":
                    if len(e.args) != 1:
                        raise LogyxError("sum accetta un solo argomento")
                    return f"({self.expr(e.args[0])}).iter().sum::<i64>()"
                if nm in ("floor", "ceil"):
                    if len(e.args) != 1:
                        raise LogyxError(f"{nm} accetta un solo argomento")
                    return f"(({self.expr(e.args[0])}) as f64).{nm}() as i64"
                if nm == "sqrt":
                    if len(e.args) != 1:
                        raise LogyxError("sqrt accetta un solo argomento")
                    return f"(({self.expr(e.args[0])}) as f64).sqrt()"
                if nm == "index_of":
                    if len(e.args) != 2:
                        raise LogyxError("index_of accetta due argomenti: index_of(stringa, sottostringa)")
                    return (f"({self.expr(e.args[0])}).find(({self.expr(e.args[1])}).as_str())"
                            ".map(|i| i as i64).unwrap_or(-1i64)")
                if nm == "substring":
                    if len(e.args) != 3:
                        raise LogyxError("substring accetta tre argomenti: substring(stringa, inizio, fine)")
                    return (f"({self.expr(e.args[0])})[({self.expr(e.args[1])}) as usize.."
                            f"({self.expr(e.args[2])}) as usize].to_string()")
                if nm == "split":
                    if len(e.args) != 2:
                        raise LogyxError("split accetta due argomenti: split(stringa, separatore)")
                    return (f"({self.expr(e.args[0])}).split(({self.expr(e.args[1])}).as_str())"
                            ".map(|x| x.to_string()).collect::<Vec<String>>()")
                if nm == "join":
                    if len(e.args) != 2:
                        raise LogyxError("join accetta due argomenti: join(lista, separatore)")
                    return f"({self.expr(e.args[0])}).join(({self.expr(e.args[1])}).as_str())"
                if nm == "replace":
                    if len(e.args) != 3:
                        raise LogyxError("replace accetta tre argomenti: replace(stringa, da, a)")
                    return (f"({self.expr(e.args[0])}).replace(({self.expr(e.args[1])}).as_str(), "
                            f"({self.expr(e.args[2])}).as_str())")
                if nm == "to_json":
                    if len(e.args) != 1:
                        raise LogyxError("to_json accetta un solo argomento")
                    ta = self._type_of(e.args[0], getattr(self, "cur_types", {}))
                    if not ta:
                        raise LogyxError("to_json: non riesco a dedurre il tipo dell'argomento")
                    self.uses_json = True
                    return self._json_value(self.expr(e.args[0]), ta)
                if nm in ("starts_with", "ends_with"):
                    if len(e.args) != 2:
                        raise LogyxError(f"{nm} accetta due argomenti: {nm}(stringa, parte)")
                    method = "starts_with" if nm == "starts_with" else "ends_with"
                    return f"({self.expr(e.args[0])}).{method}(({self.expr(e.args[1])}).as_str())"
                if nm == "now":
                    if len(e.args) != 0:
                        raise LogyxError("now non accetta argomenti")
                    return ("(std::time::SystemTime::now()"
                            ".duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)")
                if nm == "format_date":
                    if len(e.args) != 1:
                        raise LogyxError("format_date accetta un solo argomento (timestamp)")
                    self.deps["chrono"] = '"0.4"'
                    return ("chrono::DateTime::from_timestamp((" + self.expr(e.args[0])
                            + "), 0).unwrap().format(\"%Y-%m-%d %H:%M:%S\").to_string()")
                if nm == "from_json":
                    if len(e.args) != 2 or not isinstance(e.args[1], N.Identifier):
                        raise LogyxError("from_json richiede: from_json(testo, NomeRecord)")
                    rec = e.args[1].name
                    if rec not in self.records:
                        raise LogyxError(f"from_json: '{rec}' non è un record")
                    self.uses_serde = True
                    self.deps["serde"] = '{ version = "1", features = ["derive"] }'
                    self.deps["serde_json"] = '"1"'
                    return f"serde_json::from_str::<{rec}>(&({self.expr(e.args[0])})).unwrap()"
                if nm == "sha256":
                    if len(e.args) != 1:
                        raise LogyxError("sha256 accetta un solo argomento")
                    self.deps["sha2"] = '"0.10"'
                    return ("{ use sha2::{Sha256, Digest}; let mut __h = Sha256::new(); "
                            "__h.update((" + self.expr(e.args[0])
                            + ").as_bytes()); format!(\"{:x}\", __h.finalize()) }")
                if nm == "map":
                    fn = self._fn_name(e.args, 2, 1, "map(lista, funzione)")
                    return (f"({self.expr(e.args[0])}).iter().cloned().map(|x| {fn}(x))"
                            ".collect::<Vec<_>>()")
                if nm == "filter":
                    fn = self._fn_name(e.args, 2, 1, "filter(lista, funzione)")
                    return (f"({self.expr(e.args[0])}).iter().cloned().filter(|x| {fn}(x.clone()))"
                            ".collect::<Vec<_>>()")
                if nm == "reduce":
                    fn = self._fn_name(e.args, 3, 2, "reduce(lista, iniziale, funzione)")
                    return (f"({self.expr(e.args[0])}).iter().cloned()"
                            f".fold({self.expr(e.args[1])}, |acc, x| {fn}(acc, x))")
                if nm == "sort":
                    raise LogyxError("usa sort come istruzione, non dentro un'espressione")
            args = ", ".join(self._arg(a) for a in e.args)
            return f"{self.expr(e.callee)}({args})"
        raise LogyxError(f"espressione non supportata dal transpiler v0: {t}")
