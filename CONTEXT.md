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

## Flusso (ottobre 2026)

Push su GitHub **consentito**: commit locale a ogni passo + `git push origin main` (le Pages si aggiornano
da sole, build da `main`/`docs`). Suite attuale: **32/32**.

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

**Collezioni di stringhe: FATTE** (`design/collezioni-stringhe.md`); stringhe complete (`split`/`join`/
`replace`, manca solo regex).
**`map`/`filter`/`reduce`: FATTI** (Fase 1 con funzioni *nominate* — `design/funzioni-ordine-superiore.md`).
**Fase 2 (futura):** valori funzione e lambda inline (serve un tipo funzione nel linguaggio).
**Record/struct: FATTI** (`design/record.md`): `record Nome { campo: tipo }`, costruzione posizionale
`Nome(v1, v2)`, accesso `p.campo`; passabili/ritornabili dalle funzioni (clone sugli argomenti). In Rust
→ `#[derive(Clone)] struct`. Interprete + nativo, suite 28/28 (`examples/native_record.logyx`).
Limiti v0: campi scalari/string (no record annidati, no liste/mappe di record — prossimo giro di
"clone uniforme" sui contenitori con elemento record).
**JSON: serializzazione FATTA** (`design/json.md`): `to_json(x)` per int/bool/string/record (anche
annidati), formato compatto, escape; ambiente dei tipi locali aggiunto per la generazione type-directed.
Interprete+nativo, suite 29/29 (`examples/native_json.logyx`). **DA FARE:** `from_json` (parser JSON),
float/liste in `to_json`.
**`match` generale: FATTO**: `match x { <valore> { } ... else { } }` (confronto per uguaglianza →
if/else-if), distinto dal `match ok/err` degli errori; interprete+nativo, suite 30/30
(`examples/native_matchval.logyx`).
**Crates Rust: AVVIATO** (`design/crates-rust.md`): **build ibrido** — senza dipendenze `rustc` diretto,
con dipendenze crea un progetto `cargo` al volo (cartella `*_cargo/` gitignored), in entrambi i backend.
Usi reali di crate "sotto il cofano" (deterministici → conformi): **`sha256`** (crate `sha2`) e
**`from_json`** (crate `serde`/`serde_json`, con derive condizionali Serialize/Deserialize sui record).
Il transpiler/compilatore tracciano le dipendenze (`deps`, valore = spec TOML).
**Via generica alle crate: FATTA** — `use rust "crate" = "ver"` + `extern rust fn nome(p: tipo) -> tipo
= "<espr Rust>"` (binding con corpo Rust; le extern girano solo con `build`, non nell'interprete). Suite
34/34 (`examples/native_extern.logyx`, crate `hex`).
**Date/tempo: FATTO** — `now()` (timestamp Unix, via `std`, non deterministico → fuori suite) e
`format_date(ts)` (data UTC, crate `chrono`, deterministico → suite 35/35, `examples/native_date.logyx`).
Tre crate reali agganciate finora: `sha2`, `serde`, `chrono`.
**Contenitori di record: FATTI** — liste e mappe di record (`Vec<T>`/`HashMap<K,T>`) funzionano con il
"clone uniforme" già in piedi; record ora derivano anche `PartialEq` (per `contains`/`==`). Suite 36/36
(`examples/native_recordlist.logyx`).
**Web→WASM: Fase 0 FATTA** (`design/web-wasm.md`): comando `logyxc build-wasm <file>` compila le funzioni
del nucleo con firma numerica (int/float/bool) a WebAssembly (target `wasm32-unknown-unknown`, export
`#[export_name]` con conversione int→i32 per evitare i BigInt in JS) e genera una pagina HTML demo.
Verificato con Node: `raddoppia(21)=42`, `fattoriale(5)=120`, ecc. (`examples/native_wasm.logyx`).
**Web→WASM: Fase 1 FATTA** (`design/web-wasm-fase1.md`): passaggio di **stringhe** oltre il confine
JS↔WASM tramite la **memoria lineare** del modulo (NIENTE `wasm-bindgen`). Il modulo esporta
`__logyx_alloc`/`__logyx_free`; un parametro `string` diventa la coppia WASM `(ptr:i32, len:i32)` (il
wrapper ricostruisce la `String` e libera il buffer di input), un ritorno `string` è un puntatore a un
blocco `[len:u32 LE][byte UTF-8]` (JS legge e poi libera). `numeric_exports`→`wasm_exports` accetta ora
`int|float|bool|string`. Pagina HTML generata interattiva (campi di testo + eventi nel collante JS, firme
in un descrittore `SIGS`). Esempio `examples/native_webstr.logyx` (saluta/grida/lunghezza/vuoto/grado):
suite 38/38 (la logica string→string entra nella suite via `main`), lato WASM verificato con Node
(round-trip, UTF-8 con accenti, 2000 chiamate senza problemi di memoria).
**Concat `String + String` type-aware: FATTA** — il codegen ora riconosce la concatenazione di stringhe
anche quando non c'è un letterale nella catena (es. `r = r + testo` fra due `String`): `is_string_expr`/
`_is_string_expr` consultano l'ambiente dei tipi correnti (`cur_types`) oltre all'euristica `stringish`,
così si genera `format!("{}{}", …)` invece di `r + testo` (che in Rust non compila). Prima funzionava solo
nei `return`. Allineati i due backend; `ripeti(testo, volte)` reintrodotto in `native_webstr`, suite 38/38.
**Inferenza tipi dei locali: FATTA** — il tipo di ritorno si deduce ora anche quando si fa `return <locale>`
(non serve più l'annotazione `-> tipo`): `_infer_func_ret`/`infer_func_ret` raccolgono i tipi delle variabili
locali (da `Decl`/`Assign`, ricorrendo in `if`/`while`/`for`/`match`) oltre a quelli dei parametri. Suite
37/37 (`examples/native_localret.logyx`); `fattoriale` in `native_wasm` non ha più bisogno di `-> int`.
**DA FARE web→WASM:** DOM dal codice Logyx (`set text of …`, `on "click" …` via import host/`web-sys`),
Fase 2 (server HTTP + route/render), Fase 3 (isole `@start-client`, server-driven).
**Altro DA FARE:** extern con corpi Rust multi-riga; `to_json`/`from_json` per float/liste/mappe;
`enum`; lambda inline.
**Altri fronti:** date/tempo; web→WASM; `enum`; Fase 2 funzioni (lambda inline); contenitori di record.
Nota: stampare una lista intera con `print` non è supportato nel nativo (`Vec` non ha `Display`): negli
esempi si itera o si usa `sum`/`join`/`len`.

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
