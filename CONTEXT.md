# Punto di ripresa — Logyx

Aggiornato: 2026-09-28

## Come riprendere in una nuova sessione (anche su un altro PC)

1. Apri Claude Code dentro la cartella `logyx/`.
2. Leggi questo file e poi `DESIGN.md`.
3. Riprendi dal "Prossimo passo" qui sotto.

## Dove siamo

Definiti: nome, licenza, autore, scelte architetturali (`DESIGN.md`), **grammatica v0** (`GRAMMAR.md`),
esempi in `examples/`, e un **prototipo del frontend in Python** funzionante in `prototype/`
(lexer + parser + interprete). Esegue il nucleo del linguaggio; i costrutti web arriveranno dopo.

Il prototipo esegue il nucleo e ora anche il **render lato server**: `route` + `render` con interpolazione
`{expr}` ed escaping automatico; le isole `@start-client` sono rese come segnaposto. Comandi `render` e
`serve` (server HTTP) in `prototype/main.py`.

Prova: `cd prototype && python3 main.py serve ../examples/web_demo.logyx 8137` e apri http://127.0.0.1:8137/

## Deciso e bloccato

- **Nome:** Logyx — estensione `.logyx` (breve `.lgx`).
- **Autore:** Daniele Deplano (RedRider21). Nessuna firma di terzi nei sorgenti o nei commit.
- **Licenza:** AGPL-3.0. Header su ogni file di programma:
  `Copyright (C) 2026 Daniele Deplano (RedRider21)` + `SPDX-License-Identifier: AGPL-3.0-or-later`.
- **Versione 0:** Python e Rust sono impalcature temporanee. Python = prototipo usa-e-getta;
  Rust = trampolino di compilazione. L'obiettivo è un Logyx **self-hosted** (compilatore scritto in
  Logyx, backend proprio) in cui né Python né Rust restano una dipendenza.
- **Backend (v0):** transpiling verso Rust, poi `rustc` (nativo + WASM). Poi backend proprio.
- **Host del compilatore:** v0 prototipo in Python; poi compilatore self-hosted in Logyx.
- **Confine server/client:** modello server-driven di default.
- **Ecosistema:** FFI con C + interop crates Rust.

## Prossimo passo

Piano deciso (2026-09-28): **congelare la semantica del nucleo**, poi passare a Rust.
Tacca 1 — moduli/import: **FATTA**. Tacca 2 — gestione degli errori: **FATTA**.
La semantica del nucleo è ora congelata.

Compilatore in Rust (Fase 1) — crate `compiler/` (`cargo`, binario `logyxc`). **COMPLETO sul
sottoinsieme nativo e conforme al prototipo (8/8 esempi, stesso output):**
- **lexer** (`src/lexer.rs`, `src/token.rs`) — `logyxc tokens <file>`.
- **AST + parser** (`src/ast.rs`, `src/parser.rs`) — `logyxc parse <file>`.
- **risoluzione import** (`src/modules.rs`) — porting di `modules.py`.
- **inferenza tipi + backend Rust** (`src/codegen.rs`) — porting di `rustgen.py`; `logyxc gen <file>`
  stampa il Rust, `logyxc build <file>` genera il `.rs` e invoca `rustc`.
- Nota: il compilatore nativo non gestisce i costrutti web (`@`/route/render danno errore), come il
  transpiler del prototipo.

Rete di sicurezza in piedi: **`tests/conformance.sh`** confronta `python build` e `logyxc build` su
tutti gli esempi (ora **9/9**). Flusso per ogni nuova feature nativa: implementarla in interprete +
transpiler Python + compilatore Rust, aggiungere un esempio e lanciare la suite. Primo giro completato
così col builtin **`push(lista, valore)`** (`examples/native_push.logyx`).

RIPRENDI DA QUI → due strade possibili:
- **A) Self-hosting (orizzonte v0):** iniziare a riscrivere il compilatore **in Logyx stesso**, così da
  non dipendere più né da Python né da Rust. È il traguardo finale; va pianificato (quali parti per prime,
  come fare il bootstrap del binario).
- **B) Ampliare il sottoinsieme nativo** (interprete + transpiler + compilatore, mantenendo la suite
  verde): altre mutazioni (`remove`), collezioni di stringhe, iterazione su mappa; più avanti i costrutti
  web verso WASM.

In coda (rifiniture del prototipo, opzionali): collezioni di stringhe, iterazione su mappa, target WASM
del client.
(Fatto: nucleo; render lato server con `{for}`/`{if}`; isole client → JavaScript; transpiler Rust con `build`
su sottoinsieme tipizzato — stringhe e concatenazione, `range(a,b)`, divisione intera `i64`, inferenza del
tipo di ritorno **e dei tipi dei parametri**, **liste** e **mappe** di scalari; **moduli/import** con
risoluzione a caricamento; **gestione errori** (`fail`/`?`/`match` → `Result<T,String>`); esempi
`native_infer/list/map.logyx`, `use_import.logyx`+`lib_math.logyx`, `errori.logyx`.)

## Questioni aperte da decidere

- Gerarchia dei principi in caso di conflitto.
- Forma dei blocchi: v0 usa graffe `{}` (confermato); valutare eventuale stile `end`.
- Precedenze complete degli operatori e template dettagliato (da formalizzare col parser).
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il server-driven).
- API client definitiva (v0 provvisoria: `on "<evento>" of "<sel>" { }`, `set text of "<sel>" to <expr>`).
