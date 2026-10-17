#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Smoke test del database SQLite (db_open/db_exec/db_query). Verifica che i tre
# backend — interprete, transpiler (python build) e compilatore nativo (logyxc) —
# diano lo STESSO output su examples/native_db.logyx. Fuori dalla suite veloce
# perché la crate `rusqlite` (bundled) compila SQLite da sorgente (build pesante).
# Esce 0 se tutto conforme, 1 se diverge, 77 se l'ambiente non lo consente.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROTO="$ROOT/prototype"
EXAMPLES="$ROOT/examples"
LOGYXC="$ROOT/compiler/target/release/logyxc"
FILE="$EXAMPLES/native_db.logyx"

command -v cargo   >/dev/null 2>&1 || { echo "salto: 'cargo' non installato"; exit 77; }
command -v python3 >/dev/null 2>&1 || { echo "salto: 'python3' non installato"; exit 77; }

cleanup() { rm -f "$EXAMPLES"/native_db.rs "$EXAMPLES"/native_db_bin; rm -rf "$EXAMPLES"/native_db_cargo; }
trap cleanup EXIT

echo "== build del compilatore Rust (release) =="
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 1; }

exec_output() { sed -n '/esecuzione/,$p' | tail -n +2; }

echo "== esecuzione nei tre backend (la build rusqlite richiede un po') =="
INT=$( cd "$PROTO" && python3 main.py "$FILE" 2>/dev/null )
TRA=$( cd "$PROTO" && python3 main.py build "$FILE" 2>/dev/null | exec_output )
NAT=$( "$LOGYXC" build "$FILE" 2>/dev/null | exec_output )

ok=1
if [ "$INT" = "$TRA" ] && [ "$INT" = "$NAT" ]; then
    echo "  OK    native_db (interprete == transpiler == compilatore)"
else
    ok=0
    echo "  DIFF  native_db"
    echo "    --- interprete ---";  echo "$INT" | sed 's/^/    /'
    echo "    --- transpiler ---";  echo "$TRA" | sed 's/^/    /'
    echo "    --- compilatore --";  echo "$NAT" | sed 's/^/    /'
fi

echo "---------------------------------------"
if [ "$ok" = 1 ]; then echo "DB CONFORME ✅"; exit 0; else echo "DB NON conforme ❌"; exit 1; fi
