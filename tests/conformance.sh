#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Suite di conformità: verifica che il compilatore Rust (logyxc) e il prototipo
# Python producano lo STESSO output d'esecuzione su tutti gli esempi nativi.
# Esce con 0 se tutto conforme, 1 se qualcosa diverge, 2 in caso di errore di setup.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROTO="$ROOT/prototype"
EXAMPLES="$ROOT/examples"
LOGYXC="$ROOT/compiler/target/release/logyxc"

# Esempi supportati dal compilatore nativo (niente costrutti web).
CASES="native_fib native_hello native_range native_infer native_list native_map native_push native_math native_text native_remove native_contains native_trim native_pow native_breakcont native_compound native_listops native_round native_has native_sqrt native_strops native_strlist native_splitjoin native_replace native_mapfilter native_reduce native_record native_json use_import errori"

echo "== build del compilatore Rust (release) =="
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 2; }

# Estrae l'output d'esecuzione (tutto ciò che segue la riga 'esecuzione').
exec_output() { sed -n '/esecuzione/,$p' | tail -n +2; }

pass=0; fail=0; failed=""
for c in $CASES; do
    f="$EXAMPLES/$c.logyx"
    py=$( cd "$PROTO" && python3 main.py build "$f" 2>/dev/null | exec_output )
    rs=$( "$LOGYXC" build "$f" 2>/dev/null | exec_output )
    if [ "$py" = "$rs" ]; then
        echo "  OK    $c"
        pass=$((pass + 1))
    else
        echo "  DIFF  $c"
        fail=$((fail + 1)); failed="$failed $c"
        echo "    --- prototipo (python) ---"; echo "$py" | sed 's/^/    /'
        echo "    --- compilatore (rust)  ---"; echo "$rs" | sed 's/^/    /'
    fi
done

# Pulizia degli artefatti generati accanto agli esempi (sono comunque gitignored).
rm -f "$EXAMPLES"/*.rs "$EXAMPLES"/*_bin

echo "---------------------------------------"
echo "Conformi: $pass   Differenti: $fail"
if [ "$fail" -eq 0 ]; then
    echo "TUTTO CONFORME ✅"
    exit 0
else
    echo "NON conformi:$failed"
    exit 1
fi
