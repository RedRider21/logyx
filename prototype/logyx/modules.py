# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Risoluzione dei moduli: espande gli `import` prima di interprete/transpiler.

Gli `import "file.logyx"` vengono risolti a livello di caricamento: ogni file
importato viene letto una sola volta (dedup per percorso assoluto, i cicli sono
spezzati) e le sue definizioni vengono unite in un'unica lista di elementi, senza
nodi Import. Interprete e transpiler ricevono quindi il programma già completo.
"""

import os

from .lexer import Lexer
from .parser import Parser
from .errors import LogyxError
from . import nodes as N


def _read_items(path):
    with open(path, encoding="utf-8") as f:
        src = f.read()
    tokens = Lexer(src, path).tokenize()
    return Parser(tokens, path).parse()


def _resolve_path(spec, base_dir):
    cand = spec if os.path.isabs(spec) else os.path.join(base_dir, spec)
    if not os.path.exists(cand) and not cand.endswith(".logyx"):
        alt = cand + ".logyx"
        if os.path.exists(alt):
            cand = alt
    return os.path.abspath(cand)


def _expand(items, base_dir, seen, out, names):
    for it in items:
        if isinstance(it, N.Import):
            path = _resolve_path(it.path, base_dir)
            if not os.path.exists(path):
                raise LogyxError(f"import: modulo non trovato: {it.path!r}")
            if path in seen:
                continue
            seen.add(path)
            _expand(_read_items(path), os.path.dirname(path), seen, out, names)
        else:
            if isinstance(it, N.FunctionDef):
                if it.name in names:
                    raise LogyxError(
                        f"import: la funzione '{it.name}' è definita più volte "
                        "(conflitto tra moduli)"
                    )
                names[it.name] = True
            out.append(it)
    return out


def load_program(main_path):
    """Legge il file principale ed espande ricorsivamente i suoi import."""
    mp = os.path.abspath(main_path)
    items = _read_items(mp)
    return _expand(items, os.path.dirname(mp), {mp}, [], {})
