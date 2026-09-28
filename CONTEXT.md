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

RIPRENDI DA QUI → **mappe nel transpiler Rust**: tradurre i letterali di mappa `{k: v}`, l'accesso `m[k]`
e (in prospettiva) l'iterazione verso `HashMap<K, V>`, deducendo `K`/`V`. Oggi le liste di scalari sono
supportate; le mappe esistono solo nell'interprete.

Dopo, in coda: liste di stringhe / liste come parametro-ritorno, moduli/import, gestione errori nel
linguaggio, target WASM del client. Vedi `MANUAL.md` §13.
(Fatto: nucleo; render lato server con `{for}`/`{if}`; isole client → JavaScript; transpiler Rust con `build`
su sottoinsieme tipizzato — stringhe e concatenazione, `range(a,b)`, divisione intera `i64`, inferenza del
tipo di ritorno **e dei tipi dei parametri**, e **liste di scalari** (letterale/`len`/index/`for`); esempi
`native_infer.logyx` e `native_list.logyx`.)

## Questioni aperte da decidere

- Gerarchia dei principi in caso di conflitto.
- Forma dei blocchi: v0 usa graffe `{}` (confermato); valutare eventuale stile `end`.
- Precedenze complete degli operatori e template dettagliato (da formalizzare col parser).
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il server-driven).
- API client definitiva (v0 provvisoria: `on "<evento>" of "<sel>" { }`, `set text of "<sel>" to <expr>`).
