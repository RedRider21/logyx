#!/usr/bin/env bash
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Esegue ogni esempio della pagina di riferimento nei tre backend (vedi riferimento.py).
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
( cd "$ROOT/compiler" && cargo build --release ) >/dev/null 2>&1 \
    || { echo "errore: 'cargo build --release' fallito"; exit 2; }
exec python3 "$ROOT/tests/riferimento.py"
