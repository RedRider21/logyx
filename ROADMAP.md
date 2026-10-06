# Logyx — Roadmap e gap-analysis

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Aggiornato:** 2026-10-02

Elenco ragionato di ciò che manca a Logyx per diventare un linguaggio "completo", nello spirito del
confronto con PHP e JavaScript. È una mappa delle priorità, non una promessa di avere tutto subito.

## Filosofia (da `DESIGN.md`)

> Il linguaggio resta piccolo, l'ecosistema copre i mondi.

A differenza di PHP (migliaia di funzioni nel cuore del linguaggio), Logyx punta a un **nucleo piccolo e
solido** e a una **libreria standard a strati**, scritta per quanto possibile **in Logyx stesso** (che è
anche il banco di prova verso il self-hosting). Quindi "coprire tutte le funzioni di PHP/JS nel
linguaggio" non è il traguardo: il traguardo è nucleo completo + librerie.

Legenda: ✅ fatto · 🟡 parziale · ⏳ da fare.

---

## Piano 1 — Costrutti del linguaggio (vanno nel compilatore)

Senza questi non si possono nemmeno *scrivere* le librerie: hanno la priorità più alta.

| Costrutto | Stato | Priorità |
| --- | --- | --- |
| `if`/`else`, `while`, `for` (range e liste) | ✅ | — |
| Funzioni con inferenza dei tipi | ✅ | — |
| Errori come valori (`fail`/`?`/`match ok/err`) | ✅ | — |
| Moduli (`import`) | ✅ | — |
| **`break` / `continue`** nei cicli | ✅ | — |
| **Operatori composti** (`+=`, `-=`, `*=`, …) | ✅ | — |
| **Funzioni come valori / closure / lambda** | ⏳ | **1** |
| Parametri con default; numero variabile di argomenti | ⏳ | 3 |
| **Record/struct** (tipi con campi, anche in liste/mappe) | ✅ | — |
| `match` generale (sui valori, con `else`) | ✅ | — |
| `enum` (tipi somma) | ✅ varianti con/senza **payload**, costruzione `Nome.Var(args)`, `==` e `match` con binding | 3 |
| Tuple; `for` con indice | ⏳ | 4 |
| Costanti globali a primo livello; conversioni esplicite (`int()`, `float()`) | 🟡 | 3 |

## Piano 2 — Libreria standard (funzioni, a strati)

Da costruire preferibilmente **in Logyx**, non come builtin infiniti nel compilatore.

| Area | Esempi | Stato |
| --- | --- | --- |
| Stringhe | `len`, `upper`, `lower`, `trim`, `index_of`, `substring`, `split`, `join`, `replace` ✅ · regex ⏳ | 🟡 |
| Collezioni di stringhe | liste e mappe di `String` ✅ (vedi `design/collezioni-stringhe.md`) | ✅ |
| Liste | `len`, `push`, `remove`, `contains`, `sum`, `sort`, `map`, `filter`, `reduce` ✅ · slice ⏳ | 🟡 |
| Mappe | letterale, accesso, `len`, `has`, `keys`, `values`, iterazione `for k in mappa` (chiavi ordinate) ✅ | 🟢 |
| Numeri | `abs`, `min`, `max`, `pow`, `floor`, `ceil` ✅ · `sqrt`, `round`, `random` ⏳ | 🟡 |
| I/O e sistema | `print` ✅ · lettura input, file, ambiente ⏳ | 🟡 |
| Dati | **JSON**: `to_json` + `from_json` ✅ · **date/tempo**: `now`, `format_date` (crate `chrono`) ✅ | 🟡 |

## Piano 2.5 — Librerie esterne (l'ecosistema)

Logyx transpila a Rust: eredita tre porte verso le librerie esterne. **Da costruire** (oggi `import`
carica solo file `.logyx` locali).

| Porta | Cosa apre | Come | Priorità |
| --- | --- | --- | --- |
| **Crates Rust** | l'ecosistema crates.io (serde, reqwest, regex, DB, ...) | `use rust "crate"` + `extern rust fn` (binding), build ibrido `rustc`/`cargo` | ✅ **funziona** — builtin via crate (`sha256`/`from_json`) + **via generica** (`examples/native_extern.logyx`) |
| **FFI con C** | librerie di sistema / C | dichiarazione firme + linker | media |
| **Ponte Python** | numpy/pandas/ML | `PyO3` (CPython incorporato) | bassa (binario pesante, no WASM) |

Nota: il ponte Python è possibile ma costoso (perde il "puro nativo" e il target WASM) → opzione di
nicchia, non fondamento. Le crates Rust coprono quasi tutto ciò che serve.

## Piano 3 — Capacità di piattaforma (il confronto vero con PHP/JS)

Oggi esistono solo come abbozzi nel vecchio motore Python; **non** ancora nel compilatore nativo.

| Capacità | Esempi | Stato |
| --- | --- | --- |
| Web server | routing, HTTP, `render`: **nel compilatore nativo** — `logyxc render`/`serve`/`build-server` (crate `tiny_http`); sessioni/DB ⏳ | 🟢 (nativo, base) |
| Database | query, connessioni, ORM leggero | ⏳ |
| Client | isole `@start-client`→JS **nel compilatore nativo** (eventi, stato, DOM), servite con la pagina; `fetch` ⏳ | 🟢 (nativo, base) |
| Target | **WebAssembly (client): Fasi 0+1 fatte** — `logyxc build-wasm` compila a WASM funzioni con firma `int`/`float`/`bool`/`string` (le stringhe via memoria lineare, senza `wasm-bindgen`), con pagina HTML interattiva (`design/web-wasm.md`, `design/web-wasm-fase1.md`, `examples/native_wasm.logyx`, `examples/native_webstr.logyx`); DOM dal codice Logyx + server HTTP ⏳ | 🟡 |

---

## Ordine consigliato (le prossime tappe)

1. **Chiudere il nucleo** (Piano 1, priorità 1–2): `break`/`continue`, operatori composti, **closure**,
   record/struct. Sono il collo di bottiglia: abilitano tutto il resto.
2. **Libreria standard in Logyx** (Piano 2): iniziare da stringhe e liste (`split`, `join`, `map`,
   `filter`, `sort`), scritte in Logyx e verificate dalla suite di conformità.
3. **Dati**: JSON e date/tempo (servono a quasi ogni app reale).
4. **Capacità web** (Piano 3) portate nel compilatore nativo, poi **WASM** per il client.

Ogni tappa segue il ciclo già rodato: implementare nei backend (o in Logyx, per le librerie), aggiungere
un esempio, mantenere verde `tests/conformance.sh`.
