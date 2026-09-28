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
- **Backend:** transpiling verso Rust, poi `rustc` (nativo + WASM). Non LLVM diretto all'inizio.
- **Host del compilatore:** prototipo in Python/TypeScript, compilatore vero in Rust.
- **Confine server/client:** modello server-driven di default.
- **Ecosistema:** FFI con C + interop crates Rust.

## Prossimo passo

RIPRENDI DA QUI → **inferenza dei tipi dei parametri** nel transpiler Rust: dedurre il tipo dei parametri
dall'uso (aritmetica → int/float, concatenazione/interpolazione → string, confronti/logica → bool, chiamate
ad altre funzioni), così da non doverli più annotare. Oggi è dedotto solo il tipo di ritorno; i parametri
vanno ancora annotati (`rustgen.py`, `ty()` solleva errore se il tipo è assente).

Dopo, in coda: liste/mappe nel transpiler, moduli/import, gestione errori nel linguaggio, target WASM del
client. Vedi `MANUAL.md` §13.
(Fatto: nucleo; render lato server con `{for}`/`{if}`; isole client → JavaScript; transpiler Rust con `build`
su sottoinsieme tipizzato — stringhe e concatenazione, `range(a,b)`, divisione intera `i64`, inferenza del
tipo di ritorno.)

## Questioni aperte da decidere

- Gerarchia dei principi in caso di conflitto.
- Forma dei blocchi: v0 usa graffe `{}` (confermato); valutare eventuale stile `end`.
- Precedenze complete degli operatori e template dettagliato (da formalizzare col parser).
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il server-driven).
- API client definitiva (v0 provvisoria: `on "<evento>" of "<sel>" { }`, `set text of "<sel>" to <expr>`).
