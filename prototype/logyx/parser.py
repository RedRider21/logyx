# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later

from .tokens import T
from . import nodes as N
from .errors import LogyxError
from .lexer import Lexer


class Parser:
    """Discesa ricorsiva: dai token all'AST."""

    def __init__(self, tokens, filename="<input>"):
        self.toks = tokens
        self.i = 0
        self.filename = filename

    # --- utilita' ---

    def peek(self, k=0):
        return self.toks[self.i + k]

    def at(self, ttype):
        return self.peek().type == ttype

    def advance(self):
        t = self.toks[self.i]
        self.i += 1
        return t

    def expect(self, ttype, what=None):
        if not self.at(ttype):
            t = self.peek()
            self.error(f"atteso {what or ttype}, trovato {t.type} ({t.value!r})", t)
        return self.advance()

    def error(self, msg, tok=None):
        tok = tok or self.peek()
        raise LogyxError(f"{self.filename}:{tok.line}:{tok.col}: errore di sintassi: {msg}")

    # --- programma ---

    def parse(self):
        items = []
        while not self.at(T.EOF):
            if self.at(T.IMPORT):
                items.append(self.import_stmt())
            elif self.at(T.FN):
                items.append(self.function())
            elif self.at(T.IDENT) and self.peek().value == "route":
                items.append(self.route_def())
            else:
                items.append(self.statement())
        return items

    def import_stmt(self):
        self.expect(T.IMPORT)
        tok = self.expect(T.STRING, "percorso del modulo da importare")
        parts = tok.value
        if len(parts) != 1 or parts[0][0] != "lit":
            self.error("il percorso di 'import' deve essere una stringa semplice (senza interpolazione)")
        return N.Import(parts[0][1])

    def route_def(self):
        self.advance()  # 'route'
        tok = self.expect(T.STRING, "percorso della route")
        parts = tok.value
        if len(parts) != 1 or parts[0][0] != "lit":
            self.error("il percorso della route deve essere una stringa semplice (senza interpolazione)")
        path = parts[0][1]
        return N.RouteDef(path, self.block())

    def render_stmt(self):
        self.expect(T.RENDER)
        tok = self.expect(T.TEMPLATE, "template dopo 'render'")
        return N.Render(tok.value)

    def function(self):
        self.expect(T.FN)
        name = self.expect(T.IDENT, "nome di funzione").value
        self.expect(T.LPAREN)
        params, ptypes = [], []
        if not self.at(T.RPAREN):
            n, t = self.param()
            params.append(n)
            ptypes.append(t)
            while self.at(T.COMMA):
                self.advance()
                n, t = self.param()
                params.append(n)
                ptypes.append(t)
        self.expect(T.RPAREN)
        ret = None
        if self.at(T.ARROW):
            self.advance()
            ret = self.type_ref()
        body = self.block()
        return N.FunctionDef(name, params, body, ptypes, ret)

    def param(self):
        name = self.expect(T.IDENT, "nome di parametro").value
        ptype = None
        if self.at(T.COLON):
            self.advance()
            ptype = self.type_ref()
        return name, ptype

    def type_ref(self):
        # Un tipo, eventualmente fallibile:  T | error
        base = self._type_base()
        if self.at(T.PIPE):
            self.advance()
            w = self.expect(T.IDENT, "'error'")
            if w.value != "error":
                self.error(f"dopo '|' nel tipo è atteso 'error', trovato '{w.value}'")
            return base + "|error"
        return base

    def _type_base(self):
        # I tipi sono opzionali (gradual typing): li conserviamo come stringa.
        if self.at(T.LBRACK):
            self.advance()
            inner = self._type_base()
            self.expect(T.RBRACK)
            return "[" + inner + "]"
        if self.at(T.LBRACE):
            self.advance()
            k = self._type_base()
            self.expect(T.COLON)
            v = self._type_base()
            self.expect(T.RBRACE)
            return "{" + k + ": " + v + "}"
        if self.at(T.NIL):
            self.advance()
            return "nil"
        return self.expect(T.IDENT, "tipo").value

    def block(self):
        self.expect(T.LBRACE)
        stmts = []
        while not self.at(T.RBRACE) and not self.at(T.EOF):
            stmts.append(self.function() if self.at(T.FN) else self.statement())
        self.expect(T.RBRACE)
        return stmts

    # --- istruzioni ---

    def statement(self):
        t = self.peek()
        if t.type == T.IF:
            return self.if_stmt()
        if t.type == T.WHILE:
            return self.while_stmt()
        if t.type == T.FOR:
            return self.for_stmt()
        if t.type == T.RETURN:
            return self.return_stmt()
        if t.type == T.FAIL:
            return self.fail_stmt()
        if t.type == T.MATCH:
            return self.match_stmt()
        if t.type == T.BREAK:
            self.advance()
            return N.Break()
        if t.type == T.CONTINUE:
            self.advance()
            return N.Continue()
        if t.type == T.CONST:
            return self.const_decl()
        if t.type == T.RENDER:
            return self.render_stmt()
        # dichiarazione tipizzata:  IDENT ':' tipo '=' espressione
        if t.type == T.IDENT and self.peek(1).type == T.COLON:
            name = self.advance().value
            self.advance()  # ':'
            self.type_ref()
            self.expect(T.ASSIGN)
            return N.Decl(name, self.expression(), False)
        expr = self.expression()
        if self.at(T.ASSIGN):
            self.advance()
            value = self.expression()
            if isinstance(expr, (N.Identifier, N.Index)):
                return N.Assign(expr, value)
            self.error("assegnazione a un bersaglio non valido")
        compound = {
            T.PLUSEQ: "+", T.MINUSEQ: "-", T.STAREQ: "*", T.SLASHEQ: "/", T.PERCENTEQ: "%",
        }
        if self.peek().type in compound:
            op = compound[self.advance().type]
            rhs = self.expression()
            if isinstance(expr, (N.Identifier, N.Index)):
                return N.Assign(expr, N.Binary(op, expr, rhs))
            self.error("assegnazione composta a un bersaglio non valido")
        return N.ExprStmt(expr)

    def if_stmt(self):
        self.expect(T.IF)
        cond = self.expression()
        then_block = self.block()
        else_block = None
        if self.at(T.ELSE):
            self.advance()
            else_block = [self.if_stmt()] if self.at(T.IF) else self.block()
        return N.If(cond, then_block, else_block)

    def while_stmt(self):
        self.expect(T.WHILE)
        cond = self.expression()
        return N.While(cond, self.block())

    def for_stmt(self):
        self.expect(T.FOR)
        var = self.expect(T.IDENT, "variabile di ciclo").value
        self.expect(T.IN)
        iterable = self.expression()
        return N.For(var, iterable, self.block())

    def return_stmt(self):
        self.expect(T.RETURN)
        if self.at(T.RBRACE) or self.at(T.EOF):
            return N.Return(None)
        return N.Return(self.expression())

    def fail_stmt(self):
        self.expect(T.FAIL)
        return N.Fail(self.expression())

    def match_stmt(self):
        self.expect(T.MATCH)
        subject = self.expression()
        self.expect(T.LBRACE)
        ok_var = ok_block = err_var = err_block = None
        while not self.at(T.RBRACE) and not self.at(T.EOF):
            tag = self.expect(T.IDENT, "ramo 'ok' oppure 'err'").value
            if tag not in ("ok", "err"):
                self.error(f"in 'match' sono ammessi solo i rami 'ok' e 'err', trovato '{tag}'")
            var = self.expect(T.IDENT, "nome della variabile del ramo").value
            block = self.block()
            if tag == "ok":
                ok_var, ok_block = var, block
            else:
                err_var, err_block = var, block
        self.expect(T.RBRACE)
        if ok_block is None or err_block is None:
            self.error("'match' richiede entrambi i rami 'ok' e 'err'")
        return N.Match(subject, ok_var, ok_block, err_var, err_block)

    def const_decl(self):
        self.expect(T.CONST)
        name = self.expect(T.IDENT, "nome costante").value
        if self.at(T.COLON):
            self.advance()
            self.type_ref()
        self.expect(T.ASSIGN)
        return N.Decl(name, self.expression(), True)

    # --- espressioni (per precedenza crescente) ---

    def expression(self):
        return self.or_expr()

    def or_expr(self):
        left = self.and_expr()
        while self.at(T.OR):
            self.advance()
            left = N.Logical("or", left, self.and_expr())
        return left

    def and_expr(self):
        left = self.equality()
        while self.at(T.AND):
            self.advance()
            left = N.Logical("and", left, self.equality())
        return left

    def equality(self):
        left = self.comparison()
        while self.peek().type in (T.EQ, T.NE):
            op = self.advance().value
            left = N.Binary(op, left, self.comparison())
        return left

    def comparison(self):
        left = self.term()
        while self.peek().type in (T.LT, T.LE, T.GT, T.GE):
            op = self.advance().value
            left = N.Binary(op, left, self.term())
        return left

    def term(self):
        left = self.factor()
        while self.peek().type in (T.PLUS, T.MINUS):
            op = self.advance().value
            left = N.Binary(op, left, self.factor())
        return left

    def factor(self):
        left = self.unary()
        while self.peek().type in (T.STAR, T.SLASH, T.PERCENT):
            op = self.advance().value
            left = N.Binary(op, left, self.unary())
        return left

    def unary(self):
        if self.at(T.NOT):
            self.advance()
            return N.Unary("not", self.unary())
        if self.at(T.MINUS):
            self.advance()
            return N.Unary("-", self.unary())
        return self.postfix()

    def postfix(self):
        e = self.primary()
        while True:
            if self.at(T.LPAREN):
                self.advance()
                args = []
                if not self.at(T.RPAREN):
                    args.append(self.expression())
                    while self.at(T.COMMA):
                        self.advance()
                        args.append(self.expression())
                self.expect(T.RPAREN)
                e = N.Call(e, args)
            elif self.at(T.LBRACK):
                self.advance()
                idx = self.expression()
                self.expect(T.RBRACK)
                e = N.Index(e, idx)
            elif self.at(T.QUESTION):
                self.advance()
                e = N.Try(e)
            else:
                break
        return e

    def primary(self):
        t = self.peek()
        if t.type in (T.INT, T.FLOAT, T.TRUE, T.FALSE):
            self.advance()
            return N.Literal(t.value)
        if t.type == T.NIL:
            self.advance()
            return N.Literal(None)
        if t.type == T.STRING:
            self.advance()
            return self.build_string(t.value)
        if t.type == T.IDENT:
            self.advance()
            return N.Identifier(t.value)
        if t.type == T.LPAREN:
            self.advance()
            e = self.expression()
            self.expect(T.RPAREN)
            return e
        if t.type == T.LBRACK:
            self.advance()
            elements = []
            if not self.at(T.RBRACK):
                elements.append(self.expression())
                while self.at(T.COMMA):
                    self.advance()
                    if self.at(T.RBRACK):
                        break
                    elements.append(self.expression())
            self.expect(T.RBRACK)
            return N.ListLit(elements)
        if t.type == T.LBRACE:
            self.advance()
            pairs = []
            if not self.at(T.RBRACE):
                pairs.append(self.map_pair())
                while self.at(T.COMMA):
                    self.advance()
                    if self.at(T.RBRACE):
                        break
                    pairs.append(self.map_pair())
            self.expect(T.RBRACE)
            return N.MapLit(pairs)
        self.error(f"espressione attesa, trovato {t.type} ({t.value!r})", t)

    def map_pair(self):
        key = self.expression()
        self.expect(T.COLON)
        return (key, self.expression())

    def build_string(self, parts):
        out = []
        for kind, val in parts:
            if kind == "lit":
                out.append(("lit", val))
            else:
                sub = Lexer(val, self.filename).tokenize()
                p = Parser(sub, self.filename)
                node = p.expression()
                if not p.at(T.EOF):
                    self.error(f"interpolazione con espressione non valida: {{{val}}}")
                out.append(("expr", node))
        return N.StringLit(out)
