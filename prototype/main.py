#!/usr/bin/env python3
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Esegue un programma Logyx col prototipo (lexer -> parser -> interprete).

Uso:  python main.py <file.logyx>
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from logyx.lexer import Lexer
from logyx.parser import Parser
from logyx.interpreter import Interpreter
from logyx.errors import LogyxError


def main(argv):
    if len(argv) < 2:
        print("uso: python main.py <file.logyx>", file=sys.stderr)
        return 2
    path = argv[1]
    try:
        with open(path, encoding="utf-8") as f:
            src = f.read()
    except OSError as e:
        print(f"Errore: impossibile aprire {path}: {e}", file=sys.stderr)
        return 1
    try:
        tokens = Lexer(src, path).tokenize()
        items = Parser(tokens, path).parse()
        Interpreter().run(items)
    except LogyxError as e:
        print(f"Errore: {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
