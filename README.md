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
  backend scritti in Rust — la **Fase 1 del bootstrap**. Copre lo stesso sottoinsieme nativo del
  prototipo ed è **verificato conforme** (stesso output) su tutti gli esempi.

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
| `prototype/` | Interprete/transpiler in Python (lexer, parser, interprete, rustgen) |
| `compiler/` | Il compilatore vero in Rust (`logyxc`): lexer, parser, inferenza, backend |
| `tests/` | Suite di conformità (`conformance.sh`): prototipo e compilatore danno lo stesso output |
| `examples/` | Programmi di esempio `.logyx` |
| `docs/` | Documenti esportati (es. il documento di design in `.md`/`.pdf`) |
| `LICENSE` | Testo completo della licenza AGPL-3.0 |

## Prerequisiti

Il progetto è tutto qui dentro; sulla macchina servono solo gli strumenti standard:

- **Python 3.8+** — per eseguire il prototipo (nessuna dipendenza esterna).
- **Rust** (`rustc`) — *opzionale*, solo per compilare a nativo con `build`. Senza Rust, `build` genera
  comunque il codice `.rs` da compilare altrove.

## Come iniziare

Il prototipo gira con il solo Python 3.8+ (Rust è opzionale, solo per compilare a nativo):

```bash
git clone https://github.com/RedRider21/logyx.git
cd logyx/prototype

# esegui un programma
python3 main.py ../examples/hello.logyx

# avvia un server web
python3 main.py serve ../examples/web_demo.logyx 8137

# transpila a Rust e compila a nativo
python3 main.py build ../examples/native_fib.logyx
```
