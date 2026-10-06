# Logyx

Un nuovo linguaggio di programmazione: versatile e conciso come Python, comodo per il web come PHP,
ma **compilato a codice macchina** e capace di girare **sia sul server sia sul client**.
Un solo sorgente, una sola sintassi, molti bersagli (server nativo, browser via WASM, desktop, mobile).

Il nome unisce la radice greca **λόγος** (parola, ragione, linguaggio) al suono `-yx` (dalla pietra onyx).

- **Estensione dei file:** `.logyx` (breve: `.lgx`)
- **Autore:** Daniele Deplano (RedRider21)
- **Licenza:** AGPL-3.0 (vedi `LICENSE`)
- **Stato:** **versione 0** — prototipo funzionante in Python (interprete + render web + transpiler Rust su sottoinsieme tipizzato)
- **Sito:** https://redrider21.github.io/logyx/ · **Manuale online:** https://redrider21.github.io/logyx/manuale.html
- **Documento di design online:** https://claude.ai/code/artifact/0aa46f19-cef5-4728-8e26-4b83867259cd

## In due parole

Logyx nasce per **cancellare la divisione tra linguaggio server e linguaggio client**: un solo
sorgente, una sola sintassi, che gira nativo sul server e come WebAssembly nel browser. Deploy
semplice come PHP, velocità vicina al C, concisione di Python. Il backend **transpila verso Rust**
e da lì `rustc` raggiunge nativo, WASM, desktop e mobile.

## Come funziona (compilatore, server, client)

Se sei nuovo, il quadro d'insieme in poche righe:

- **Il compilatore `logyxc` gira sulla tua macchina** (come `gcc`/`rustc`): non si installa sul server né
  nel browser. Prende il sorgente `.logyx` e **produce artefatti**, diversi secondo il comando:
  `build` → eseguibile nativo; `build-server`/`serve` → **binario server HTTP** (da tenere su un server);
  `build-wasm` → `.wasm` + `.html` per il **browser**; `render` → stampa l'HTML di una route (debug).
- **Un sorgente, due nature.** Il **codice server** (`route`, `render`, funzioni) finisce nel binario
  server e gira sul server; le **isole client** (`@start-client … @end-client`) sono compilate a
  JavaScript e **inviate nella pagina** al browser, dove girano (clic, DOM, stato).
- **A runtime**: l'utente apre il browser → richiesta HTTP → il server rende l'HTML (con dentro lo
  `<script>` dell'isola) → il browser mostra la pagina ed esegue il JS. Stesso schema di PHP/Next.js, ma
  server e client **dallo stesso file e linguaggio**.
- **Online**: copi il binario su un server e lo avvii; è autosufficiente e il browser non installa nulla.
  In locale provi già tutto con `logyxc serve app.logyx 8080` → `http://127.0.0.1:8080`.

(`build-wasm` è un caso a parte: porta nel browser funzioni di **puro calcolo** come WebAssembly e può
girare anche **senza server**, servendo i file statici.)

## Siamo alla versione 0

Logyx è alla **versione 0**. Il prototipo in **Python** e la transpilazione verso **Rust** sono
impalcature temporanee per far nascere il linguaggio e sperimetrarne la semantica. L'orizzonte è un
Logyx **self-hosted** — un compilatore scritto in Logyx stesso, che genera direttamente codice
macchina — in cui **né Python né Rust resteranno una dipendenza**. Sono i ponteggi, non l'edificio.

## Cosa funziona già (prototipo)

- **Nucleo**: variabili, funzioni, `if`/`while`/`for`, liste e mappe, stringhe con
  interpolazione, builtin (`print`, `len`, `str`, `range`).
- **Moduli**: `import "file.logyx"` per riusare funzioni fra file (dedup e cicli gestiti).
- **Gestione errori** come valori (modello Result): `fail`, propagazione con `?`,
  `match { ok v { } err e { } }`.
- **Web lato server**: `route` + `render` con interpolazione, escaping automatico e blocchi
  di controllo `{ for }` / `{ if }`.
- **Isole client** `@start-client … @end-client` compilate a **JavaScript**.
- **Transpiler Rust** (comando `build`) su un sottoinsieme tipizzato: aritmetica, confronti,
  logica, stringhe, `if`/`while`/`for`, ricorsione, **liste** e **mappe** di scalari, **moduli**
  e **gestione errori** (`-> T | error` → `Result<T, String>`). I tipi di **ritorno e dei parametri**
  sono **dedotti** dall'uso. `rustc` compila il `.rs` prodotto a eseguibile nativo.
- **Compilatore vero in Rust** (`compiler/`, binario `logyxc`): lexer, parser, inferenza dei tipi e
  backend scritti in Rust — la **Fase 1 del bootstrap**. Oltre al nucleo nativo (verificato **conforme**
  al prototipo su tutti gli esempi) compila a **WebAssembly** (`build-wasm`), rende l'HTML lato server
  (`render`), serve via **HTTP** (`serve`/`build-server`, crate `tiny_http`) e compila le **isole client**
  `@start-client` a JavaScript, servite con la pagina.

Il dettaglio aggiornato di ciò che il prototipo esegue è nel `MANUAL.md`; il punto di ripresa
dello sviluppo è in `CONTEXT.md`.

## Intestazione di copyright per i file di programma

Ogni file sorgente del progetto deve iniziare con:

```
Copyright (C) 2026 Daniele Deplano (RedRider21)
SPDX-License-Identifier: AGPL-3.0-or-later
```

## Com'è organizzata questa cartella

| File / cartella | Contenuto |
| --- | --- |
| `README.md` | Questo file: cos'è Logyx e come riprendere |
| `MANUAL.md` | Manuale del linguaggio e delle funzioni (aggiornato) |
| `GRAMMAR.md` | Grammatica v0 (EBNF) |
| `DESIGN.md` | Appunti di design completi e portabili |
| `CONTEXT.md` | Punto di ripresa: dove siamo e prossimo passo |
| `ROADMAP.md` | Gap-analysis e priorità: cosa manca verso un linguaggio completo |
| `design/` | Mini-design dei fronti grossi (collezioni, record, JSON, crate, web→WASM…) |
| `prototype/` | Interprete/transpiler in Python (lexer, parser, interprete, rustgen) |
| `compiler/` | Il compilatore vero in Rust (`logyxc`): lexer, parser, inferenza, backend |
| `tests/` | Suite di conformità (`conformance.sh`): prototipo e compilatore danno lo stesso output |
| `examples/` | Programmi di esempio `.logyx` |
| `docs/` | Sito (GitHub Pages) e documenti esportati (es. il documento di design in `.md`/`.pdf`); git li versiona col resto, così restano con te quando cloni il progetto |
| `LICENSE` | Testo completo della licenza AGPL-3.0 |

### Dentro `compiler/` (il compilatore Rust `logyxc`)

Fase 1 del bootstrap: il compilatore vero, in **Rust**. Build con `cargo build --release`
(→ `compiler/target/release/logyxc`).

| File | Ruolo |
| --- | --- |
| `src/token.rs` | Tipi di token |
| `src/lexer.rs` | Lexer (sorgente → token), inclusa la cattura dei template web dopo `render` |
| `src/ast.rs` | Albero sintattico |
| `src/parser.rs` | Parser a discesa ricorsiva (token → AST) e compilazione delle isole client `@start-client` → JavaScript |
| `src/modules.rs` | Risoluzione degli `import` |
| `src/codegen.rs` | Inferenza dei tipi e generazione del codice Rust (nativo, WASM, render/server) |
| `src/error.rs` | Tipo d'errore con posizione |
| `src/main.rs` | CLI: `build`/`gen`/`tokens`/`parse`/`build-wasm`/`render`/`build-server`/`serve` |

### Dentro `prototype/` (il frontend in Python)

Prima implementazione eseguibile (lexer → parser → interprete tree-walking), più il transpiler verso
Rust. Solo Python 3.8+, nessuna dipendenza. Convenzione: se il file definisce `fn main()`, viene chiamata
automaticamente.

| File | Ruolo |
| --- | --- |
| `logyx/lexer.py` | Sorgente → token (commenti, interpolazione, template) |
| `logyx/tokens.py` | Tipi di token e parole chiave |
| `logyx/parser.py` | Token → AST (discesa ricorsiva) |
| `logyx/nodes.py` | Nodi dell'AST |
| `logyx/interpreter.py` | Esecuzione dell'AST (ambiti, funzioni, builtin, render delle route) |
| `logyx/client.py` | Compila le isole `@start-client` in JavaScript |
| `logyx/rustgen.py` | Transpiler: AST → Rust (comando `build`) |
| `logyx/modules.py` | Risoluzione degli `import` |
| `logyx/errors.py` | Tipo d'errore |
| `main.py` | CLI del prototipo: esegui · `render` · `serve` · `build` |

## Prerequisiti

Il progetto è tutto qui dentro; sulla macchina servono solo gli strumenti standard:

- **Python 3.8+** — per eseguire il prototipo (nessuna dipendenza esterna).
- **Rust** (`rustc`) — *opzionale*, solo per compilare a nativo con `build`. Senza Rust, `build` genera
  comunque il codice `.rs` da compilare altrove.

## Come iniziare

Due modi: il **prototipo Python** (per provare al volo, senza Rust) e il **compilatore nativo `logyxc`**
(la via completa: nativo, WebAssembly, server HTTP). Fanno le stesse cose sul nucleo e sono verificati
conformi; i comandi di `logyxc` sono nella tabella qui sotto.

```bash
git clone https://github.com/RedRider21/logyx.git
cd logyx

# --- prototipo Python (solo Python 3.8+, nessuna dipendenza) ---
cd prototype
python3 main.py ../examples/hello.logyx                 # esegue un programma
python3 main.py serve ../examples/web_demo.logyx 8137   # server web di prova
cd ..

# --- compilatore nativo logyxc (richiede Rust) ---
cd compiler && cargo build --release && cd ..           # costruisce ./compiler/target/release/logyxc
./compiler/target/release/logyxc build examples/native_fib.logyx   # compila a nativo ed esegue
./compiler/target/release/logyxc serve examples/web_demo.logyx 8137 # server HTTP → http://127.0.0.1:8137
```

## Comandi del compilatore nativo (`logyxc`)

Compila il binario una volta con `cd compiler && cargo build --release`
(→ `compiler/target/release/logyxc`); negli esempi `logyxc` sta per quel percorso (mettilo nel `PATH`,
o usa `./compiler/target/release/logyxc`). Dalla radice del repo:

| Comando | Cosa fa |
|---|---|
| `logyxc build <file>` | Transpila a Rust, compila a nativo ed esegue |
| `logyxc gen <file>` | Stampa il codice Rust generato |
| `logyxc tokens <file>` / `logyxc parse <file>` | Diagnostica: token / AST |
| `logyxc build-wasm <file>` | Compila a WebAssembly le funzioni `int`/`float`/`bool`/`string` + pagina HTML interattiva |
| `logyxc render <file> <percorso>` | Rende l'HTML di una `route` (lato server) e lo stampa |
| `logyxc build-server <file>` | Compila un server HTTP (crate `tiny_http`) e stampa il path del binario |
| `logyxc serve <file> [porta]` | Compila e avvia il server HTTP (default porta 8080) |

```bash
logyxc build       examples/native_fib.logyx
logyxc build-wasm  examples/native_webstr.logyx   # → .wasm + .html (apri servendo la cartella)
logyxc render      examples/web_demo.logyx /       # stampa l'HTML della route "/"
logyxc serve       examples/web_demo.logyx 8137    # http://127.0.0.1:8137
```
