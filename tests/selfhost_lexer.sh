#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Self-hosting, test a specchio del LEXER: il lexer scritto in Logyx
# (selfhost/lexer.logyx) deve produrre gli STESSI token di `logyxc tokens` su ogni
# sorgente .logyx del progetto (esempi + il lexer stesso).
#
# Il lexer Logyx gira in due modi: con l'interprete Python e compilato a nativo da
# logyxc (vero self-hosting: Logyx che compila il lexer di Logyx).
#   ./tests/selfhost_lexer.sh            entrambi i modi
#   ./tests/selfhost_lexer.sh --nativo   solo il binario nativo (veloce)
# Esce con 0 se tutto identico, 1 se qualcosa diverge, 2 in caso di errore di setup.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGYXC="$ROOT/compiler/target/release/logyxc"
LEXER="$ROOT/selfhost/lexer.logyx"
PROG="$ROOT/selfhost/tokens.logyx"
BIN="$ROOT/selfhost/tokens_bin"
SOLO_NATIVO=0
[ "${1:-}" = "--nativo" ] && SOLO_NATIVO=1

echo "== build del compilatore Rust (release) =="
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 2; }

echo "== compilazione nativa del lexer Logyx (logyxc build) =="
( cd "$ROOT/selfhost" && "$LOGYXC" build tokens.logyx ) >/dev/null 2>&1
[ -x "$BIN" ] || { echo "errore: compilazione di selfhost/tokens.logyx fallita"; exit 2; }

FILES=$(ls "$ROOT"/examples/*.logyx "$ROOT"/selfhost/*.logyx)
pass=0; fail=0; failed=""
for f in $FILES; do
    nome="${f#$ROOT/}"
    atteso=$( "$LOGYXC" tokens "$f" 2>&1 )
    nat=$( "$BIN" "$f" 2>&1 )
    ok=1
    [ "$nat" = "$atteso" ] || ok=0
    if [ $SOLO_NATIVO -eq 0 ] && [ $ok -eq 1 ]; then
        int=$( cd "$ROOT/prototype" && python3 main.py "$PROG" "$f" 2>&1 )
        [ "$int" = "$atteso" ] || ok=0
    fi
    if [ $ok -eq 1 ]; then
        echo "  OK    $nome"
        pass=$((pass + 1))
    else
        echo "  DIFF  $nome"
        fail=$((fail + 1)); failed="$failed $nome"
        diff <(echo "$atteso") <(echo "$nat") | head -6 | sed 's/^/    /'
    fi
done

rm -f "$ROOT"/selfhost/*.rs "$BIN"; rm -rf "$ROOT"/selfhost/*_cargo
echo "---------------------------------------"
echo "Identici: $pass   Differenti: $fail"
if [ $fail -eq 0 ]; then
    echo "LEXER SELF-HOSTED CONFORME ✅"
    exit 0
fi
echo "NON identici:$failed"
exit 1
