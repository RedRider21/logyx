#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Smoke test del client WASM (web -> WASM, Fasi 0/1): compila a WebAssembly gli
# esempi del client e verifica, con Node, il COLLANTE JS REALE delle pagine
# generate (vedi tests/wasm_smoke.mjs). Esce 0 se tutto passa, 1 se qualcosa
# diverge, 77 se l'ambiente non consente il test (dipendenze assenti).
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXAMPLES="$ROOT/examples"
LOGYXC="$ROOT/compiler/target/release/logyxc"
CASES="native_wasm native_webstr"

command -v node >/dev/null 2>&1 || { echo "salto: 'node' non installato"; exit 77; }
command -v cargo >/dev/null 2>&1 || { echo "salto: 'cargo' non installato"; exit 77; }
rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown \
    || { echo "salto: target 'wasm32-unknown-unknown' non installato (rustup target add wasm32-unknown-unknown)"; exit 77; }

echo "== build del compilatore Rust (release) =="
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 1; }

cleanup() {
    for c in $CASES; do
        rm -rf "$EXAMPLES/$c.wasm" "$EXAMPLES/$c.html" "$EXAMPLES/${c}_wasm"
    done
}
trap cleanup EXIT

echo "== compilazione a WASM =="
for c in $CASES; do
    "$LOGYXC" build-wasm "$EXAMPLES/$c.logyx" >/dev/null 2>&1 \
        || { echo "errore: build-wasm di $c fallito"; exit 1; }
    echo "  generato $c.wasm + $c.html"
done

echo "== verifica del collante JS delle pagine =="
node "$ROOT/tests/wasm_smoke.mjs"
