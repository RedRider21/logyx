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

## ⚠️ Promemoria operativo (disposizione utente, ottobre 2026)

**Committare SOLO in locale. NON pushare su GitHub (né repo né Pages) fino a nuova indicazione esplicita
dell'utente.** Al momento ci sono **9 commit locali in attesa di push** (`git log origin/main..HEAD`).
Quando l'utente darà l'ok: `git push origin main` (e le Pages si aggiornano da sole, build da `main`/`docs`).

## RIPRENDI DA QUI — cose da fare

Si sta procedendo con l'opzione **B/C: ampliare il linguaggio a giri incrementali**, ognuno nei tre backend
(interprete + transpiler Python + compilatore Rust) con un esempio e la suite `tests/conformance.sh` verde
(ora **19/19**). Ciclo: implementa → esempio `examples/native_*` → `./tests/conformance.sh`.

**Allineamento doc a ogni aggiunta:** aggiornare `MANUAL.md`, poi rigenerare il manuale online con
`python3 tools/build_manual.py` (→ `docs/manuale.html`), e aggiungere la voce nella tabella/sezione
"Riferimento" di `docs/index.html` (con le chiavi i18n IT/EN). Il sito ha: landing + `manuale.html`.

**Già fatto in questa fase:** costrutti `break`/`continue`, operatori composti `+= -= *= /= %=`;
builtin `push`, `remove`, `contains`, `trim`, `pow`, `sum`, `sort`, `floor`, `ceil`, `abs/min/max`,
`upper/lower`. Vedi `ROADMAP.md` per il quadro completo e le priorità.

**Prossimi giri a basso rischio (C):**
- numerici: `sqrt` (scegliere esempio con risultati puliti), eventuale `round` (⚠️ Python usa arrotondamento
  bancario, Rust half-away-from-zero: allineare la semantica nei due backend o evitarlo);
- mappe: `has(mappa, chiave)`, `keys(mappa)`, `values(mappa)`, iterazione `for k in mappa`;
- stringhe: `index_of`, `substring`.

**Fronti grossi del nucleo (Piano 1 di `ROADMAP.md`), da affrontare con un minimo di design:**
- **closure / funzioni come valori** (implicazioni ownership/lifetime nel backend Rust);
- **record/struct** (tipi con campi) → sbloccano JSON e modellazione dati;
- `enum` e `match` generale (oggi `match` è solo `ok`/`err`).

**Collezioni di stringhe: FATTE** (opzione "clone uniforme", `design/collezioni-stringhe.md`): liste e
mappe di `String` ora supportate. Prossimo sblocco immediato: `split`/`join`.
**Ancora bloccati dalle closure:** `map`/`filter`/`reduce`.

**Orizzonte:** completato il nucleo + una libreria standard scritta in Logyx, puntare al **self-hosting**
(compilatore in Logyx), poi capacità web → WASM.

(Fatto in precedenza: nucleo; render lato server con `{for}`/`{if}`; isole client → JavaScript; compilatore
Rust `logyxc` conforme al prototipo; inferenza dei tipi di ritorno e parametri; liste e mappe di scalari;
moduli/import; gestione errori `fail`/`?`/`match` → `Result<T,String>`.)

## Questioni aperte da decidere

- Gerarchia dei principi in caso di conflitto.
- Forma dei blocchi: v0 usa graffe `{}` (confermato); valutare eventuale stile `end`.
- Precedenze complete degli operatori e template dettagliato (da formalizzare col parser).
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il server-driven).
- API client definitiva (v0 provvisoria: `on "<evento>" of "<sel>" { }`, `set text of "<sel>" to <expr>`).
