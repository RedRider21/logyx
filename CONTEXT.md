# Punto di ripresa — Logyx

Aggiornato: 2026-09-28

## Come riprendere in una nuova sessione (anche su un altro PC)

1. Apri Claude Code dentro la cartella `logyx/`.
2. Leggi questo file e poi `DESIGN.md`.
3. Riprendi dal "Prossimo passo" qui sotto.

## Dove siamo

Fase di **design**, avanzata. Definiti: nome, licenza, autore, scelte architetturali (vedi `DESIGN.md`),
**grammatica v0** (`GRAMMAR.md`) e i primi esempi in `examples/` (`hello.logyx`, `hello_web.logyx`).
Nessun codice del compilatore ancora scritto.

## Deciso e bloccato

- **Nome:** Logyx — estensione `.logyx` (breve `.lgx`).
- **Autore:** Daniele Deplano (RedRider21). Nessuna firma di terzi nei sorgenti o nei commit.
- **Licenza:** AGPL-3.0. Header su ogni file di programma:
  `Copyright (C) 2026 Daniele Deplano (RedRider21)` + `SPDX-License-Identifier: AGPL-3.0-or-later`.
- **Backend:** transpiling verso Rust, poi `rustc` (nativo + WASM). Non LLVM diretto all'inizio.
- **Host del compilatore:** prototipo in Python/TypeScript, compilatore vero in Rust.
- **Confine server/client:** modello server-driven di default.
- **Ecosistema:** FFI con C + interop crates Rust.

## Prossimo passo

Scrivere il **prototipo del frontend** (lexer + parser + interprete tree-walking) in Python o TypeScript,
capace di eseguire `examples/hello.logyx`. Serve a "sentire" il linguaggio prima del compilatore Rust.

## Questioni aperte da decidere

- Gerarchia dei principi in caso di conflitto.
- Forma dei blocchi: v0 usa graffe `{}` (confermato); valutare eventuale stile `end`.
- Precedenze complete degli operatori e template dettagliato (da formalizzare col parser).
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il server-driven).
- API client definitiva (v0 provvisoria: `on "<evento>" of "<sel>" { }`, `set text of "<sel>" to <expr>`).
