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

Compilatore in Rust (Fase 1) — IN CORSO, crate `compiler/` (`cargo`, binario `logyxc`):
- **lexer** (`src/lexer.rs`, `src/token.rs`): FATTO — `logyxc tokens <file>`.
- **AST + parser** (`src/ast.rs`, `src/parser.rs`): FATTO — `logyxc parse <file>`; parsa tutti gli
  esempi nativi. Nota: il compilatore nativo non gestisce i costrutti web (`@`/route/render danno errore).

RIPRENDI DA QUI → **type-checker + backend Rust** nel crate `compiler/`: riportare in Rust la logica di
`prototype/logyx/rustgen.py` (inferenza dei tipi di ritorno e parametri, liste/mappe di scalari, moduli,
gestione errori → `Result<T,String>`) per generare codice Rust dall'AST, e un comando `logyxc build
<file>` che invochi `rustc`. Verifica di conformità: stesso output del prototipo sugli esempi
`examples/native_*`, `use_import`, `errori`. Manca ancora: risoluzione degli `import` in Rust (porting di
`modules.py`). Il transpiler-a-Rust del prototipo resta il riferimento. Poi, orizzonte v0: self-hosting
(compilatore in Logyx) e uscita di scena di Python e Rust.

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
