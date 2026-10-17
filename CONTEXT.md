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

> ✅ **Variabili e costanti globali FATTE (2026-10-17, sera):** nei tre backend. Interprete già le aveva;
> nel backend tipizzato realizzate con `thread_local! RefCell` + getter `__get_X()`/setter `__set_X()`;
> accessor in `expr` solo se il nome non è un locale/param in scope (guardia via `cur_types`); errore su
> riassegnazione di `const`; v0 esclude globali di tipo collezione/tupla. Esempio `native_globals`.
>
> **Prossimo (analisi di copertura, in ordine):** **operatore
> ternario** (`cond ? a : b`) → **`in` come operatore** (`x in lista`) → esponente (`**`) → bitwise
> (`& | ^ << >> ~`) → indicizzazione stringhe/char (`s[i]`) → tipo **`set`** → comprehension → libreria
> **matematica** (`sin`/`cos`/`log`/`exp`). **Scelte di design (non buchi):** OOP/metodi, generici,
> concorrenza, argomenti con nome, spread. Tutto dettagliato in `ROADMAP.md` (Piano 1/2 + sezione
> "Scelte di design da valutare").
>
> **Prossimo fronte grosso concordato:** self-hosting — prima pietra = lexer di Logyx scritto in Logyx
> + test a specchio contro `logyxc tokens` (vedi fondo sezione / proposta in chat).

Si sta procedendo con l'opzione **B/C: ampliare il linguaggio a giri incrementali**, ognuno nei tre backend
(interprete + transpiler Python + compilatore Rust) con un esempio e la suite `tests/conformance.sh` verde
(ora **65/65**, + smoke test `db_smoke.sh`). Ciclo: implementa → esempio `examples/native_*` →
`./tests/conformance.sh`.

**`in` come operatore FATTO (2026-10-17 sera):** `item in coll` → bool (lista→`.iter().any`, mappa→
`.contains_key`, stringa→`.contains`; JS `includes`/`in`; interprete `in` di Python). Aggiunto al livello
`comparison` (nodo `Expr::In`/`N.In`); nessun conflitto col `for … in` (il `for` consuma il proprio `in`).
Esempio `native_inop`.

**Operatore ternario FATTO (2026-10-17 sera):** `cond ? a : b` (stile C; `?`/`if` scelti valutando che il
lexer **scarta i newline** → niente terminatori di statement, quindi `a if c else b` romperebbe
`x=f()` + `if`). Disambiguazione dal `?` di propagazione: in postfisso `?` è ternario **solo se seguito da
un inizio-espressione** (`starts_expr`), altrimenti `Try`; l'esistente `expr?` è sempre a fine espressione
(prima di `}`/newline) → nessuna regressione. Nodo `Expr::Ternary`/`N.Ternary`; gen `(if c { a } else { b })`,
JS `(c ? a : b)`; inferenza via `scan_expr`/`_scan_expr`. Esempio `native_ternary`. **Nota doc:** il `;`
NON è supportato (errore lessicale) — corretto nel manuale.

**Globali FATTE (2026-10-17 sera):** `const NOME = …` e variabili globali mutabili a primo livello, nei
tre backend. Tipizzati: `thread_local! RefCell` + `__get_X()`/`__set_X()`; `Item::Stmt(Decl/Assign)` al
top-level raccolto come globale; `type_of`/`expr` degli Ident con fallback/accessor ai globali; shadowing
locale gestito (guardia `cur_types`). Esempio `native_globals`.

**Database SQLite FATTO (2026-10-17):** tre builtin — `db_open(path)` (anche `":memory:"`) → handle
opaco `DB` (`Rc<rusqlite::Connection>` in Rust, oggetto `sqlite3` nell'interprete), `db_exec(db, sql)` →
righe modificate (int), `db_query(db, sql, [Record])` → lista di record (colonne→campi per posizione,
come `from_json`; il 3° arg è un tipo, gestito come in `from_json`/`_db_query`). Nativo via crate
`rusqlite` (bundled, ~31s la prima build) → tenuto FUORI dalla suite veloce, verificato da
`tests/db_smoke.sh` (3 backend identici). Esempio `examples/native_db.logyx`. **Migliorato anche il `for`
regolare:** ora tipizza la variabile di ciclo dall'elemento (`list_elem`/`map_val`/range), così
`p.campo` e le concatenazioni di stringhe funzionano sulle liste di record (da `db_query`/`from_json`).
Prossimi fronti grossi: DOM-da-Logyx in WASM, self-hosting.

**DOM-da-Logyx in WASM FATTO (2026-10-17):** tre builtin **solo-WASM** `dom_get(sel)->string` /
`dom_set(sel,txt)` / `dom_log(txt)` che chiamano funzioni **importate** dal lato JS (modulo `env`,
`#[link(wasm_import_module="env")]`) via la memoria lineare esistente (stesso ABI len-prefix). Nel
`Codegen`: flag `wasm`/`uses_dom` (setter `set_wasm_mode`); in `build`/interprete i `dom_*` danno errore
chiaro. `main.rs`: blocco `extern` import + helper `__logyx_recv_str`, **funzioni void** ora esportabili
(ret `"void"`), pagina generata con **sandbox DOM** (`#in`/`#out`) + oggetto `imports.env` (readStr/
writeStr). Esempio `examples/native_wasmdom.logyx`; verificato dal **collante JS reale** della pagina in
`tests/wasm_smoke.mjs` (DOM simulato, 18/18). **Resta solo il self-hosting** come fronte grosso.

**`fetch` dal client + `respond` lato server FATTI (2026-10-17):** chiude il ciclo "un sorgente,
server+client". Isola client: `fetch <url> [method <e>] [body <e>] [as <var>] { … } [else { … }]` →
`fetch().then(r=>r.text()).then(var=>{…}).catch(…)`, identico nei due compilatori client (`client.py` e
sezione client di `parser.rs`). Server: `respond <espr>` risponde con una stringa e `Content-Type:
application/json` (per le API); le route ora ritornano `(String, content_type)` — `__render_path` e i due
main aggiornati (nel prototipo `_Response`+`render_route` ritornano `(body, ct)`, `main.py` usa il ct).
`Stmt::Respond`/`N.Respond`. Esempio `examples/web_fetch.logyx` (pagina + route API), in suite come render
di 2 route; **verificato dal vivo** con `serve`+curl (content-type JSON vs HTML, 404). Prossimi fronti
grossi: database, DOM-da-Logyx in WASM, self-hosting.

**Tuple FATTE (2026-10-17):** letterale `(a, b, …)` (≥2), tipo `(T1, T2, …)` → tupla Rust, ritorni
multipli e **destructuring** `x, y = f()` (`Stmt::Destructure`/`N.Destructure`, `let (a,b)=…` nei backend
tipizzati, unpack nell'interprete). Nuovi `Expr::Tuple`/`N.TupleLit`; helper `tuple_parts`/`_tuple_parts`.
v0: niente accesso `t.0` (si usa il destructuring). Esempio `native_tuple`. **Con questo il Piano 1
"comodità sintattiche" è completo** (default+varargs+for-indice+lambda-var+tuple). Restano solo i fronti
grossi: database, `fetch` dal client, DOM-da-Logyx in WASM, self-hosting.

**Varargs FATTI (2026-10-17):** `fn f(a, resto: tipo...)`; l'ultimo parametro raccoglie gli argomenti
in coda in una lista `[tipo]` (al call-site diventano `vec![…]` nei backend tipizzati; nell'interprete
una `list`). Solo l'ultimo, niente default. `Param`/`FunctionDef` estesi con `variadic`/`param_variadic`;
registro `variadic` in codegen/rustgen. Esempio `native_varargs`. **Chiude il Piano 1 "comodità" tranne le
tuple.** Dipendeva dalle collezioni-come-parametro (fatte poco prima).

**Collezioni come parametro/ritorno FATTE (2026-10-17):** liste `[T]` e mappe `{K: V}` ora si passano
alle funzioni e si restituiscono anche nei backend tipizzati (`rust_type`/`ty` estesi a `Vec<T>`/
`HashMap<K,V>` con helper `list_inner`/`map_inner`; i parametri collezione registrano `kinds`+`list_elem`/
`map_val` così index/len/has/for funzionano). Rimosso il limite storico del manuale §12. Resta solo la
lista vuota `[]` non tipizzabile. Esempio `native_collparams`. **Sblocca i varargs.**

**Parametri con default FATTI (2026-10-17):** `fn f(a, b = valore)`; gli argomenti in coda si omettono e
assumono il default (riempiti **al call-site** nei tre backend), i parametri con default vanno in coda
(validato nel parser), il tipo non annotato è dedotto dal default. `Param`/`FunctionDef` estesi con
`default`/`param_defaults`; registro `defaults`+riempimento in codegen/rustgen; interprete riempie i
mancanti valutando il default. Esempio `native_defaults`. Resta del Piano 1 solo: **varargs** e **tuple**.

**Lambda in variabile FATTA (2026-10-17):** `f: fn(int)->int = |x| x+1` poi `f(5)`. Nel compilatore/
transpiler serve l'**annotazione di tipo** `fn(...)->...` (l'interprete funziona anche senza). Nodo `Decl`
esteso con `ty`/`decl_type`; la RHS lambda è generata con `fn_value_arg` (Rc<dyn Fn>), la chiamata con
`(*f)(...)`. Esempio `native_lambdavar`. Chiude il punto "lambda nuda" del Piano 1.

**Processi esterni FATTI (2026-10-17):** builtin `run(cmd)` → stdout (stringa) e `run_code(cmd)` →
codice d'uscita (int), eseguiti via `sh -c`, output **catturato** (nativo `std::process`, interprete
`subprocess`). Deterministici con comandi fissi → in suite (`examples/native_proc.logyx`). Chiude Piano 2
I/O e sistema (✅).

**`for` con indice FATTO (2026-10-17):** sintassi `for i, x in …` (indice intero 0-based + elemento),
su liste/`range`/chiavi di mappa; nei tre backend via `.enumerate()` (nel nativo/transpiler il
contatore `usize` è riportato a `i64` in testa al corpo). Nodo `For` esteso con campo opzionale `index`.
Esempio `examples/native_forindex.logyx`. Restano comodità sintattiche: tuple, parametri con default +
varargs, lambda "nuda" in variabile libera.

**Allineamento doc a ogni aggiunta:** aggiornare `MANUAL.md`, poi rigenerare il manuale online con
`python3 tools/build_manual.py` (→ `docs/manuale.html`), aggiungere la voce nella tabella/sezione
"Riferimento" di `docs/index.html` (con le chiavi i18n IT/EN) **e** nel reference completo
`docs/riferimento.html` (array `E`, bilingue it/en — vedi sotto). Il sito ha: landing (`index.html`) +
reference (`riferimento.html`) + manuale (`manuale.html`).

**Reference professionale `docs/riferimento.html` (FATTO 2026-10-17):** pagina stand-alone bilingue IT/EN
(stesso toggle lingua/tema e CSS della home), data-driven — tutte le voci sono nell'array JS `E`
(91 voci in 18 categorie `CATS`): ogni voce ha `{cat, name, sig, it, en, code, out?, opts?, lang?}`.
Barra laterale con scroll-spy, ricerca live, evidenziatore di sintassi Logyx/shell in JS. **Per aggiungere
un comando/builtin:** inserisci una voce nell'array `E` con la categoria giusta (firma con `&lt;`/`&gt;`
per i `<>`, codice con entità HTML che l'evidenziatore de-escapa). Linkata dalla home (bottone in sezione
Riferimento + nav).

**Già fatto in questa fase:** costrutti `break`/`continue`, operatori composti `+= -= *= /= %=`;
builtin `push`, `remove`, `contains`, `trim`, `pow`, `sum`, `sort`, `floor`, `ceil`, `abs/min/max`,
`upper/lower`, `sqrt`, `round`, `slice`, `random`, `matches` (regex). Vedi `ROADMAP.md` per il quadro
completo e le priorità.

**Builtin di libreria standard: COMPLETATI (Piano 2 di `ROADMAP.md`).**
- numerici: `sqrt`, `round` (⚠️ allineato *half-away-from-zero*: in Python `floor(x+0.5)`/`ceil(x-0.5)`,
  in Rust `f64::round`, non il banker's di Python — `examples/native_round.logyx`), `random`
  (crate `rand`, non deterministico → fuori suite, `examples/native_random.logyx`);
- liste: `slice(lista, a, b)` → nuova lista copia (`examples/native_slice.logyx`);
- stringhe: `index_of`, `substring`, `matches(s, pattern)` regex (crate `regex`, Python `re.search`,
  `examples/native_regex.logyx`);
- mappe: `has`, `keys`, `values`, **iterazione `for k in mappa`** (chiavi **ordinate** → deterministico;
  `examples/native_mapkeys.logyx`, `examples/native_formap.logyx`). Capitolo mappe completo.

**Fronti grossi del nucleo (Piano 1 di `ROADMAP.md`), da affrontare con un minimo di design:**
- **closure / funzioni come valori** (implicazioni ownership/lifetime nel backend Rust);
- **record/struct** (tipi con campi) → sbloccano JSON e modellazione dati;
- `enum` e `match` generale (oggi `match` è solo `ok`/`err`).

**Collezioni di stringhe: FATTE** (`design/collezioni-stringhe.md`); stringhe complete (`split`/`join`/
`replace`/`matches` regex).
**`map`/`filter`/`reduce`: FATTI** (Fase 1 con funzioni *nominate* — `design/funzioni-ordine-superiore.md`).
**Fase 2 (futura):** valori funzione e lambda inline (serve un tipo funzione nel linguaggio).
**Record/struct: FATTI** (`design/record.md`): `record Nome { campo: tipo }`, costruzione posizionale
`Nome(v1, v2)`, accesso `p.campo`; passabili/ritornabili dalle funzioni (clone sugli argomenti). In Rust
→ `#[derive(Clone)] struct`. Interprete + nativo, suite 28/28 (`examples/native_record.logyx`).
Limiti v0: campi scalari/string (no record annidati, no liste/mappe di record — prossimo giro di
"clone uniforme" sui contenitori con elemento record).
**JSON: serializzazione FATTA** (`design/json.md`): `to_json(x)` per int/**float**/bool/string/record
(anche annidati), **liste** (di qualunque tipo supportato, record e liste annidate inclusi) e **mappe**
(v0: solo chiavi string), formato compatto, escape; ambiente dei tipi locali per la generazione
type-directed. Float via helper condiviso `__json_float` (finiti interi → `3.0`; NaN/∞ rifiutati).
Liste: tipo elemento dedotto da lista letterale o variabile-lista-letterale (mappa `list_elem`), ramo
`list<ELEM>` in `json_value` (ricorsivo). Mappe: tipo valore dedotto da letterale o variabile
(mappa `map_val`), ramo `map<K,V>`, chiavi ordinate via helper `__json_obj` (HashMap Rust non
deterministico; UTF-8 byte-order == code-point order Python → conformi). Tre backend allineati
(interprete + transpiler + compiler), suite 38/38 (`examples/native_json.logyx`).
**`from_json`: FATTO per record e liste di record** — `from_json(testo, Record)` → record,
`from_json(testo, [Record])` → `Vec<Record>` (serde); il Decl registra `list_elem` per la lista-record
così è usabile (`len`, `to_json`). `examples/native_fromjson.logyx` (round-trip verificato). **`from_json` esteso a scalari e liste di
scalari**: `from_json(testo, int|float|bool|string)`, `from_json(testo, [tipo])` (helper `json_arg_type`
+ `rust_json_target`; interprete `_json_build` ricorsivo). **`from_json` di mappe FATTO**
(`from_json(testo, {string: tipo})` → `HashMap`, con `map_val` tracciato per `to_json`/`has`). **JSON
completo**: `to_json` e `from_json` coprono scalari/record/liste/mappe. **DA FARE JSON:** solo chiavi di
mappa non-string.
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
in un descrittore `SIGS`). Esempio `examples/native_webstr.logyx`
(saluta/grida/lunghezza/vuoto/grado/ripeti/etichetta, con `str()` int→string e concat):
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
**Web→WASM Fase 1 CHIUSA e verificata:** smoke test versionato `tests/wasm_smoke.sh`
(+ `tests/wasm_smoke.mjs`) che compila gli esempi client a WASM ed esegue con Node il **collante JS
reale** delle pagine generate (DOM simulato minimale), 13/13 su numeri+stringhe (round-trip, UTF-8
accentato). Salta (codice 77) se mancano node/cargo/target wasm32.
**Web Fase 2a FATTA** (`design/web-wasm-fase2.md`): il compilatore nativo ora conosce il lato **server**
`route "/" { … render <template> }`. Comando `logyxc render <file> <path>` genera un binario server e
stampa l'HTML reso, **identico al prototipo** (`python main.py render`): interpolazione `{expr}`
type-directed + HTML-escaping (`__html_escape`). Catena: lexer (nuovo token `Template`, cattura grezza
bilanciando i tag), AST (`Item::Route`, `Stmt::Render`), parser (`route_def`/`render_stmt`,
`parse_expression` pubblico per le interpolazioni), codegen (`route_fn` → `__route_N()`, `main`
dispatcher sul path). Conformità: caso `render web_demo /` nella suite (ora **39/39**). Isole
`@start-client` e `{for …}`/`{if …}` nel template → errore chiaro (fasi dopo).
**Web Fase 2b FATTA** (`design/web-wasm-fase2.md`): server HTTP nativo con crate **`tiny_http`**.
`logyxc build-server <file>` compila il server (stampa il path del binario, non avvia); `logyxc serve
<file> [porta]` compila e avvia in foreground (default 8080). Codegen: le route diventano
`__render_path(path) -> Option<String>` + un `main` server (`SERVER_MAIN`) che dispatcha, risponde
`text/html; charset=utf-8` o 404; la porta è l'argomento del binario. Modalità scelta da
`Codegen::set_server_mode` (render vs server). Smoke test versionato `tests/server_smoke.sh` (avvia,
confronta il body HTTP con `render`, verifica Content-Type e 404). `*_server/` gitignored.
**Web Fase 2c FATTA** (`design/web-wasm-fase2.md`): le isole client `@start-client … @end-client` del
template si compilano a **JavaScript** (blocco `<script>` IIFE con prelude range/len/str/print) e sono
iniettate nella pagina resa, servita dal server. Porting di `prototype/logyx/client.py` in
`compiler/src/parser.rs` (`compile_island` + `ClientCompiler`, riusa il `Parser` per le espressioni;
`js_expr` Expr→JS). Scelta: il **DOM** si fa in JS (il WASM resta per il calcolo). Output JS identico al
prototipo: `render hello_web /` nella suite (ora **40/40**), il server serve la pagina con lo `<script>`.
**Template `{for}`/`{if}` FATTI**: `{for <var> in <lista|range> { … }}` e `{if <cond> { … } else { … }}`
(anche `else if`) nel render, ricorsivi (corpo = template). Nel codegen `render_for_hole`/`render_if_hole`
+ `split_block`/`for_iter`; il tipo dell'elemento del `for` va in `cur_types` per le interpolazioni del
corpo. Esempio `examples/web_loop.logyx`, conforme al prototipo (suite **41/41**). Con questo il render
server-side del compilatore è a parità col prototipo.
**DA FARE web→WASM:** servire asset statici (il `.wasm` del client, se si vorrà calcolo WASM nel
browser); DOM dal codice Logyx via `wasm-bindgen`; Fase 3 (server-driven, WebSocket + diff del DOM).
**`enum`: FATTO** — varianti con/senza **payload**: `enum Forma { Cerchio(float), Rettangolo(float,float),
Punto }`. Accesso/costruzione `Nome.Var` o `Nome.Var(args)`; confronto `==` e `match`. Il match su enum
**con payload** è un nuovo `Stmt::MatchEnum`/`N.MatchEnum` (rami `Variante(bind…)` che **legano** i campi):
il parser lo riconosce quando un pattern è `Call(Ident, [soli Ident])`; gli enum **senza** payload restano
su `MatchValue` (confronto `==`). Codegen: `self.enums` = varianti con tipi payload; costruzione →
`E::V(args)`; `MatchEnum` → `match … { E::V(b1,b2) => … _ => … }` con i bind in `cur_types` (tipi dal
payload). Tre backend, `examples/native_enum.logyx` + `native_enumpayload.logyx`, suite **45/45**.
**DA FARE enum:** varianti con payload **nominale** (campi con nome) e match annidati, se serviranno.
**Lambda inline: FATTE** — `|p1, p2| espressione` (corpo-espressione) come argomento di
`map`/`filter`/`reduce`, in alternativa al nome di funzione. `Expr::Lambda`/`N.Lambda`; nel codegen una
closure Rust uniforme `|__p0,__p1| { let a=__p0.clone(); … body }` (il clone gestisce anche il `&T` di
`filter`). Fuori da map/filter/reduce → errore (niente tipo-funzione di prima classe in v0). Tre backend,
`examples/native_lambda.logyx`, suite **46/46**.
**I/O su file: FATTO (v0)** — builtin `read_file(path)` → string (vuota se non leggibile),
`write_file(path, testo)` → bool, `file_exists(path)` → bool; in Rust via `std::fs`/`std::path`,
nell'interprete via `open`/`os.path`. Tre backend, `examples/native_fileio.logyx` (scrive in `/tmp`,
rilegge), suite **47/47**. Apre la strada all'uso reale delle librerie di file.
**stdin / ambiente: FATTO (v0)** — `read_line()` (una riga, senza newline finale; "" su EOF),
`read_all()` (tutto lo stdin), `env(nome)` (variabile d'ambiente, "" se assente). In Rust `std::io::stdin`
/ `std::env::var`, nell'interprete `sys.stdin` / `os.environ`. Non deterministici → **fuori dalla suite**
(come `now()`): `examples/native_stdinenv.logyx`, verificato a mano (interprete == nativo con stdin/env
reali; Rust generato identico fra transpiler e compiler).
**args() (argomenti CLI): FATTO (v0)** — `args()` → lista di stringhe (argomenti dell'eseguibile, escluso
il nome del programma). In Rust `std::env::args().skip(1).collect::<Vec<String>>()`; nell'interprete
`interp.program_args` popolato da `main.py` con gli argv dopo il file. Non deterministico → fuori suite,
`examples/native_args.logyx` (verificato: interprete == nativo con `alfa beta` → `2/alfa/beta`).
**Valori funzione: FATTO (v0 parziale)** — tipo funzione `fn(T,…) -> R` (parser `type_base`), in Rust
`std::rc::Rc<dyn Fn(…) -> …>` (clonabile → clone uniforme ok). Si **passa** una funzione nominata o una
lambda a un parametro `fn(…)->…` e si **chiama** (`g(x)` → `(*g)(args)`). Nel codegen: `parse_fn_type`,
`fn_value_arg` (lambda → `Rc::new(move |p: T| body) as Rc<dyn Fn>`, nome → `Rc::new(nome) as …`, var
funzione → `.clone()`). L'interprete li gestisce nativamente (valori callable). **Ritorno di funzioni (closure): FATTO** — una
funzione con ret `fn(…)->…` può restituire una lambda/nome (campo `cur_ret`; nel `Return` si applica
`fn_value_arg`); la lambda cattura l'ambiente (`move`), e il risultato è memorizzabile via inferenza dal
ret (`g = sommatore(5)` → `cur_types[g]` tipo funzione → `(*g)(…)`). Tre backend,
`examples/native_funval.logyx` + `examples/native_closure.logyx`, suite **49/49**. **DA FARE valori
funzione:** assegnare una lambda "nuda" a una variabile libera (servirebbe il tipo annotato sul `Decl`).
**Ergonomia pratica (emersi 2026-10-16): FATTI.**
- **Assegnazione a indice** `xs[i] = v` / `m[k] = v`: FATTO in tutti i backend (lista → indicizzazione,
  mappa → `.insert`; nel codegen `kinds` distingue lista/mappa). `examples/native_indexset.logyx`.
- **Conversione stringa→numero**: FATTO — builtin `int(x)` / `float(x)` (da stringa con parse → 0/0.0 se
  non valido; fra int/float con cast). type-directed. `examples/native_convert.logyx`.
**Altro DA FARE:** extern con corpi Rust multi-riga;
valori funzione memorizzabili in variabili libere / ritorno; processi esterni.
**Cybersecurity — hardening build: FATTO (primo passo)** — `logyxc build --harden <file>` blinda il
binario: `panic=abort`, `overflow-checks=on`, strip dei simboli (rustc), e per cargo anche `lto`+
`codegen-units=1` (profilo release). Output invariato, binario molto più piccolo (fib: 4.5 MB → 350 KB).
Inoltre `--harden` scrive il **checksum** `<bin>.sha256` (via `sha256sum`, formato `sha256sum -c`) per la
**verifica d'integrità** (anti-tampering onesto: checksum esterno pubblicabile; rileva le manomissioni).
Prossimi passi cyber (vedi memoria `logyx_cybersecurity_idee`): firma crittografica (Ed25519/minisign),
build riproducibili, supply chain. Punto forte già presente: memory safety ereditata da Rust.
**DA FARE (verso la fine) — richiesta utente:** **documentazione professionale sul sito (Pages)** che
spieghi **ogni comando/builtin** con **opzioni** ed **esempio pratico**, come i linguaggi noti (stile
reference completo, non solo tabella). Oggi il sito ha una tabella sintetica; serve una pagina reference
ricca (per comando: sintassi, parametri, esempio, output).
**DA FARE (al compilatore finale) — richiesta utente:** sezione completa "Librerie esterne" su README,
sito (Pages) e manuale: **come** si caricano le librerie (oggi `use rust`/`extern rust`; in futuro binding
pronti) e un **elenco completo** delle librerie utilizzabili per categoria (file, grafica, gestione
finestre, matematica, database di vari tipi, rete, audio, ecc.). Prima versione orientativa già nel
manuale (§ "Quali librerie si possono usare"); da completare/rendere definitiva quando il compilatore è
maturo.
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
