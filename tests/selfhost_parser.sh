#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Self-hosting, test a specchio del PARSER: il parser scritto in Logyx
# (selfhost/parser.logyx, via parse.logyx) deve produrre lo STESSO albero di `logyxc parse` su ogni
# sorgente .logyx del progetto (esempi + i sorgenti del self-hosting).
#
# Il parser Logyx gira in due modi: con l'interprete Python e compilato a nativo da
# logyxc (vero self-hosting: Logyx che compila il parser di Logyx).
#   ./tests/selfhost_parser.sh            entrambi i modi
#   ./tests/selfhost_parser.sh --nativo   solo il binario nativo (veloce)
# Esce con 0 se tutto identico, 1 se qualcosa diverge, 2 in caso di errore di setup.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOGYXC="$ROOT/compiler/target/release/logyxc"
PROG="$ROOT/selfhost/parse.logyx"
BIN="$ROOT/selfhost/parse_bin"
SOLO_NATIVO=0
[ "${1:-}" = "--nativo" ] && SOLO_NATIVO=1

echo "== build del compilatore Rust (release) =="
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 2; }

echo "== compilazione nativa del parser Logyx (logyxc build) =="
( cd "$ROOT/selfhost" && "$LOGYXC" build parse.logyx ) >/dev/null 2>&1
[ -x "$BIN" ] || { echo "errore: compilazione di selfhost/parse.logyx fallita"; exit 2; }

FILES=$(ls "$ROOT"/examples/*.logyx "$ROOT"/selfhost/*.logyx)
pass=0; fail=0; failed=""
for f in $FILES; do
    nome="${f#$ROOT/}"
    atteso=$( "$LOGYXC" parse "$f" 2>&1 )
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
    echo "PARSER SELF-HOSTED CONFORME ✅"
    exit 0
fi
echo "NON identici:$failed"
exit 1
