#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Smoke test del server HTTP (web Fase 2b): compila il server con `build-server`
# (crate tiny_http), lo avvia su una porta di test, verifica che la pagina servita
# via HTTP coincida con l'output di `logyxc render` e che una route assente dia 404.
# Esce 0 se tutto passa, 1 se qualcosa diverge, 77 se l'ambiente non lo consente.
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLES="$ROOT/examples"
LOGYXC="$ROOT/compiler/target/release/logyxc"
FILE="$EXAMPLES/web_demo.logyx"
PORT=18724

command -v cargo >/dev/null 2>&1 || { echo "salto: 'cargo' non installato"; exit 77; }
command -v curl  >/dev/null 2>&1 || { echo "salto: 'curl' non installato"; exit 77; }

echo "== build del compilatore Rust (release) =="
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 1; }

echo "== compilazione del server (build-server, scarica/compila tiny_http) =="
BIN=$("$LOGYXC" build-server "$FILE" 2>/dev/null | sed -n 's|^// server compilato: ||p')
[ -x "$BIN" ] || { echo "errore: binario del server non prodotto"; exit 1; }

SRV=""
cleanup() {
    [ -n "$SRV" ] && kill "$SRV" 2>/dev/null
    rm -rf "$EXAMPLES"/web_demo.rs "$EXAMPLES"/web_demo_bin "$EXAMPLES"/web_demo_server
}
trap cleanup EXIT

echo "== avvio del server sulla porta $PORT =="
"$BIN" "$PORT" & SRV=$!
up=0
for _ in $(seq 1 100); do
    if curl -s -o /dev/null "http://127.0.0.1:$PORT/"; then up=1; break; fi
    sleep 0.1
done
[ "$up" = 1 ] || { echo "errore: il server non risponde"; exit 1; }

body=$(curl -s "http://127.0.0.1:$PORT/")
rendered=$("$LOGYXC" render "$FILE" / 2>/dev/null)
code404=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$PORT/non-esiste")
ctype=$(curl -s -D - -o /dev/null "http://127.0.0.1:$PORT/" | tr -d '\r' | sed -n 's/^[Cc]ontent-[Tt]ype: //p')

ok=1
if [ "$body" = "$rendered" ]; then
    echo "  OK    GET / coincide con 'render'"
else
    echo "  FAIL  GET / diverge da 'render'"; ok=0
    diff <(echo "$rendered") <(echo "$body") | sed 's/^/    /'
fi
case "$ctype" in
    text/html*) echo "  OK    Content-Type: $ctype" ;;
    *) echo "  FAIL  Content-Type inatteso: '$ctype'"; ok=0 ;;
esac
if [ "$code404" = "404" ]; then
    echo "  OK    route assente -> 404"
else
    echo "  FAIL  route assente -> $code404 (atteso 404)"; ok=0
fi

echo "---------------------------------------"
[ "$ok" = 1 ] && { echo "SERVER OK ✅"; exit 0; } || { echo "SERVER: differenze"; exit 1; }
