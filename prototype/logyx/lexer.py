# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later

from .tokens import Token, T, KEYWORDS
from .errors import LogyxError


class Lexer:
    """Trasforma il sorgente in una lista di token.

    Le stringhe con interpolazione ("Ciao {nome}") producono un token STRING il
    cui valore e' una lista di parti: ("lit", testo) oppure ("expr", sorgente).
    """

    def __init__(self, src, filename="<input>"):
        self.src = src
        self.i = 0
        self.line = 1
        self.col = 1
        self.filename = filename
        self.tokens = []

    def error(self, msg):
        raise LogyxError(f"{self.filename}:{self.line}:{self.col}: errore lessicale: {msg}")

    def peek(self, k=0):
        j = self.i + k
        return self.src[j] if j < len(self.src) else ""

    def advance(self):
        c = self.src[self.i]
        self.i += 1
        if c == "\n":
            self.line += 1
            self.col = 1
        else:
            self.col += 1
        return c

    def tokenize(self):
        while self.i < len(self.src):
            c = self.peek()
            if c in " \t\r\n":
                self.advance()
                continue
            if c == "/" and self.peek(1) == "/":
                while self.i < len(self.src) and self.peek() != "\n":
                    self.advance()
                continue
            if c == "/" and self.peek(1) == "*":
                self.advance()
                self.advance()
                while self.i < len(self.src) and not (self.peek() == "*" and self.peek(1) == "/"):
                    self.advance()
                if self.i < len(self.src):
                    self.advance()
                    self.advance()
                continue
            if c == "@":
                self.error(
                    "costrutti web/client (@..., route, render) non sono supportati dal "
                    "prototipo v0; usa il nucleo del linguaggio (vedi examples/hello.logyx, "
                    "examples/demo.logyx)"
                )
            if c == '"':
                self._string()
                continue
            if c.isdigit():
                self._number()
                continue
            if c.isalpha() or c == "_":
                self._ident()
                continue
            self._operator()
        self.tokens.append(Token(T.EOF, None, self.line, self.col))
        return self.tokens

    def _number(self):
        sl, sc = self.line, self.col
        start = self.i
        while self.peek().isdigit():
            self.advance()
        is_float = False
        if self.peek() == "." and self.peek(1).isdigit():
            is_float = True
            self.advance()
            while self.peek().isdigit():
                self.advance()
        text = self.src[start:self.i]
        if is_float:
            self.tokens.append(Token(T.FLOAT, float(text), sl, sc))
        else:
            self.tokens.append(Token(T.INT, int(text), sl, sc))

    def _ident(self):
        sl, sc = self.line, self.col
        start = self.i
        while self.peek().isalnum() or self.peek() == "_":
            self.advance()
        text = self.src[start:self.i]
        if text == "render":
            self.tokens.append(Token(T.RENDER, "render", sl, sc))
            self._template()
            return
        ttype = KEYWORDS.get(text, T.IDENT)
        if ttype == T.TRUE:
            self.tokens.append(Token(T.TRUE, True, sl, sc))
        elif ttype == T.FALSE:
            self.tokens.append(Token(T.FALSE, False, sl, sc))
        elif ttype == T.NIL:
            self.tokens.append(Token(T.NIL, None, sl, sc))
        else:
            self.tokens.append(Token(ttype, text, sl, sc))

    def _string(self):
        sl, sc = self.line, self.col
        self.advance()  # apre virgolette
        parts = []
        buf = []
        escapes = {"n": "\n", "t": "\t", '"': '"', "\\": "\\", "{": "{", "}": "}"}
        while True:
            if self.i >= len(self.src):
                self.error("stringa non terminata")
            c = self.peek()
            if c == '"':
                self.advance()
                break
            if c == "\\":
                self.advance()
                e = self.advance()
                buf.append(escapes.get(e, e))
                continue
            if c == "{":
                self.advance()
                if buf:
                    parts.append(("lit", "".join(buf)))
                    buf = []
                expr = []
                while self.i < len(self.src) and self.peek() != "}":
                    expr.append(self.advance())
                if self.i >= len(self.src):
                    self.error("interpolazione non terminata (manca '}')")
                self.advance()  # chiude }
                src = "".join(expr).strip()
                if not src:
                    self.error("interpolazione vuota {}")
                parts.append(("expr", src))
                continue
            buf.append(self.advance())
        if buf:
            parts.append(("lit", "".join(buf)))
        self.tokens.append(Token(T.STRING, parts, sl, sc))

    def _operator(self):
        sl, sc = self.line, self.col
        two = self.peek() + self.peek(1)
        two_map = {
            "->": T.ARROW, "==": T.EQ, "!=": T.NE, "<=": T.LE, ">=": T.GE,
            "+=": T.PLUSEQ, "-=": T.MINUSEQ, "*=": T.STAREQ, "/=": T.SLASHEQ, "%=": T.PERCENTEQ,
        }
        if two in two_map:
            self.advance()
            self.advance()
            self.tokens.append(Token(two_map[two], two, sl, sc))
            return
        c = self.peek()
        singles = {
            "(": T.LPAREN, ")": T.RPAREN, "{": T.LBRACE, "}": T.RBRACE,
            "[": T.LBRACK, "]": T.RBRACK, ",": T.COMMA, ":": T.COLON,
            "=": T.ASSIGN, "+": T.PLUS, "-": T.MINUS, "*": T.STAR,
            "/": T.SLASH, "%": T.PERCENT, "<": T.LT, ">": T.GT,
            "?": T.QUESTION, "|": T.PIPE, ".": T.DOT,
        }
        if c in singles:
            self.advance()
            self.tokens.append(Token(singles[c], c, sl, sc))
            return
        self.error(f"carattere inatteso {c!r}")

    # --- cattura del template dopo 'render' ---

    def _template(self):
        """Cattura in modo grezzo il template HTML che segue 'render'.

        Delimita il template contando la profondita' dei tag, saltando i buchi
        di interpolazione {...} e le isole @start-client ... @end-client.
        """
        while self.peek() in " \t\r\n":
            self.advance()
        if self.peek() != "<":
            self.error("dopo 'render' e' atteso un template che inizia con '<'")
        sl, sc = self.line, self.col
        start = self.i
        depth = 0
        started = False
        while self.i < len(self.src):
            c = self.peek()
            if c == "{":
                self._skip_braces()
                continue
            if c == "@" and self.src.startswith("@start-client", self.i):
                self._skip_island()
                continue
            if c == "<":
                closing, selfclose = self._consume_tag()
                if closing:
                    depth -= 1
                elif not selfclose:
                    depth += 1
                    started = True
                elif selfclose and depth == 0:
                    started = True
                if started and depth == 0:
                    break
                continue
            self.advance()
        raw = self.src[start:self.i]
        self.tokens.append(Token(T.TEMPLATE, raw, sl, sc))

    def _consume_tag(self):
        self.advance()  # '<'
        closing = self.peek() == "/"
        last = ""
        while self.i < len(self.src) and self.peek() != ">":
            last = self.advance()
        if self.i >= len(self.src):
            self.error("tag del template non terminato (manca '>')")
        self.advance()  # '>'
        return closing, last == "/"

    def _skip_braces(self):
        self.advance()  # '{'
        depth = 1
        while self.i < len(self.src) and depth > 0:
            c = self.advance()
            if c == "{":
                depth += 1
            elif c == "}":
                depth -= 1
        if depth > 0:
            self.error("interpolazione non terminata nel template (manca '}')")

    def _skip_island(self):
        self._consume_literal("@start-client")
        end = "@end-client"
        while self.i < len(self.src) and not self.src.startswith(end, self.i):
            self.advance()
        if self.i >= len(self.src):
            self.error("isola client non terminata (manca @end-client)")
        self._consume_literal(end)

    def _consume_literal(self, lit):
        for _ in range(len(lit)):
            self.advance()
