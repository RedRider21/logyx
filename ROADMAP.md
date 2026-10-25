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
| **Funzioni come valori / closure / lambda** | ✅ lambda `|a,b| expr`; **valori funzione** (`Rc<dyn Fn>`) passabili, chiamabili, **restituiti** (closure che catturano l'ambiente) e **assegnabili a variabile** con tipo annotato (`f: fn(int)->int = |x| x+1`) | — |
| Parametri con default; numero variabile di argomenti | ✅ **default** `p = valore` (in coda); **varargs** `p: tipo...` (ultimo, raccoglie in lista) | — |
| **Record/struct** (tipi con campi, anche in liste/mappe) | ✅ | — |
| `match` generale (sui valori, con `else`) | ✅ | — |
| `enum` (tipi somma) | ✅ varianti con/senza **payload**, costruzione `Nome.Var(args)`, `==` e `match` con binding | 3 |
| Tuple; `for` con indice | ✅ **tuple** `(a, b)` con tipo `(T1, T2)`, ritorni multipli e destructuring `x, y = f()`; **`for` con indice** `for i, x in …` | — |
| **Variabili e costanti globali** (a primo livello, fuori dalle funzioni) | ✅ nei tre backend — `const NOME = …` (immutabile) e variabili globali mutabili; accesso da qualunque funzione, un locale/param omonimo le oscura. Nel backend tipizzato: `thread_local! RefCell` + getter/setter. v0: non ancora globali di tipo collezione/tupla | — |
| Conversioni esplicite (`int()`, `float()`) | ✅ | — |
| **Operatore condizionale (ternario)** `cond ? a : b` | ✅ nei tre backend — espressione, annidabile a destra; `?` disambiguato dalla propagazione errori (ternario solo se seguito da un valore). `examples/native_ternary.logyx` | — |
| **`in` come operatore booleano** (`x in lista`, `k in mappa`, `sub in stringa`) | ✅ nei tre backend — distinto dal `for … in`; lista→contiene, mappa→chiave, stringa→sottostringa. `examples/native_inop.logyx` | — |
| **Operatore di esponente** `**` | ✅ nei tre backend — più stretto di `*`/`/`, destra-assoc; interi→`pow`, float→`powf`. `examples/native_power.logyx` | — |
| **Operatori bit a bit** `& \| ^ << >> ~` | ✅ nei tre backend — su interi, precedenza come in C; il `\|` bit a bit non confligge con la lambda `\|x\|` né col tipo `T \| error` (posizioni diverse). `examples/native_bitwise.logyx` | — |
| **Indicizzazione di stringhe** (`s[i]`) | ✅ nei tre backend — `s[i]` → carattere come stringa di 1. `examples/native_strindex.logyx`. (Tipo carattere/bytes a sé: ⏳) | — |
| **Tipo `set`** (insieme, elementi unici) | ✅ nei tre backend — letterale `{a, b, …}` (distinto dalla mappa), `add`/`remove` per valore, `in`, `len`, `for` ordinato (`BTreeSet`). `examples/native_set.logyx` | — |
| **List comprehension** (`[x*2 for x in xs if …]`) | ✅ nei tre backend — `[espr for var in iterabile (if cond)?]` → lista; itera range/lista/set/chiavi-mappa in ordine deterministico; distinta dalla lista letterale dal `for` dopo il primo elemento. `examples/native_comprehension.logyx` | — |

## Piano 2 — Libreria standard (funzioni, a strati)

Da costruire preferibilmente **in Logyx**, non come builtin infiniti nel compilatore.

| Area | Esempi | Stato |
| --- | --- | --- |
| Stringhe | `len`, `upper`, `lower`, `trim`, `index_of`, `substring`, `split`, `join`, `replace`, `matches` (regex) ✅ | ✅ |
| Collezioni di stringhe | liste e mappe di `String` ✅ (vedi `design/collezioni-stringhe.md`) | ✅ |
| Liste | `len`, `push`, `remove`, `contains`, `sum`, `sort`, `map`, `filter`, `reduce`, `slice`, assegnazione `xs[i]=v` ✅ | ✅ |
| Mappe | letterale, accesso, `len`, `has`, `keys`, `values`, iterazione `for k in mappa` (chiavi ordinate) ✅ | 🟢 |
| Numeri | `abs`, `min`, `max`, `pow`, `floor`, `ceil`, `sqrt`, `round`, `random`, conversioni `int`/`float` ✅ · **libreria matematica** `sin`/`cos`/`tan`, `log`/`log10`/`exp`, costanti `pi()`/`e()` ✅ | ✅ |
| I/O e sistema | `print`, **file** (`read_file`/`write_file`/`file_exists`), **stdin** (`read_line`/`read_all`), **ambiente** (`env`), **argomenti CLI** (`args`), **processi esterni** (`run`/`run_code`) ✅ | ✅ |
| Dati | **JSON**: `to_json` + `from_json` (scalari/record/liste/mappe) ✅ · **date/tempo**: `now`, `format_date` (crate `chrono`) ✅ | ✅ |

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
| Web server | routing, HTTP, `render` (HTML) e **`respond`** (stringa/JSON, Content-Type dedicato): **nel compilatore nativo** — `logyxc render`/`serve`/`build-server` (crate `tiny_http`); **database SQLite** già integrato (riga sotto); sessioni ⏳ | 🟢 (nativo) |
| Database | **SQLite**: `db_open`/`db_exec`/`db_query(…, [Record])` (righe tipizzate via record) ✅; altri DB/ORM ⏳ | 🟢 (SQLite) |
| Client | isole `@start-client`→JS **nel compilatore nativo** (eventi, stato, DOM), servite con la pagina; **`fetch`** (chiamate HTTP dall'isola, aggiornamento DOM con la risposta) ✅ | 🟢 (nativo) |
| Target | **WebAssembly (client): Fasi 0+1+2 fatte** — `logyxc build-wasm` compila a WASM funzioni `int`/`float`/`bool`/`string` (stringhe via memoria lineare, senza `wasm-bindgen`), pagina HTML interattiva, e **DOM dal codice Logyx** (`dom_get`/`dom_set`/`dom_log` via import JS del modulo `env`) — `examples/native_wasm`/`native_webstr`/`native_wasmdom`; eventi DOM dal WASM ⏳ | 🟢 |

---

## Scelte di design da valutare (non "buchi": decisioni di paradigma)

Elementi che **alcuni** linguaggi famosi hanno e Logyx no — ma che sono scelte di stile, non lacune del
nucleo. Da decidere se e quando adottarli (dopo il nucleo e, in parte, dopo il self-hosting).

| Elemento | Note / direzione possibile |
| --- | --- |
| **OOP classica**: classi, ereditarietà, **metodi** `x.metodo()`, interfacce | Oggi Logyx è **funzionale + record/enum** (stile Rust/Go). Possibile: aggiungere i **metodi** sui record (sintassi `x.metodo()`), e/o un sistema a **trait/interfacce** invece dell'ereditarietà. |
| **Generici / polimorfismo parametrico** (`fn<T>`) | Utile per contenitori e librerie riusabili; impatta forte l'inferenza dei tipi. Tipicamente roba da v1+. |
| **Concorrenza**: thread, `async`/`await`, canali | Grande capitolo; eredita molto da Rust. Da valutare dopo il self-hosting. |
| **Argomenti con nome** al call-site (`f(x=1)`) e **spread** `*args` in chiamata | Comodità ergonomiche, non essenziali al nucleo. |

## Ordine consigliato (le prossime tappe)

1. **Nucleo "da ogni linguaggio" (Piano 1): COMPLETO ✅** — variabili/costanti globali ✅, operatore
   ternario ✅, `in` come operatore ✅, esponente `**` ✅, bitwise ✅, indicizzazione stringhe ✅,
   libreria matematica ✅, tipo `set` ✅, comprehension ✅. Prossimo grande passo → **self-hosting**.
2. **Libreria matematica** (Piano 2): `sin`/`cos`/`log`/`exp`, costanti.
3. **Self-hosting** (orizzonte): lexer di Logyx scritto in Logyx + test a specchio contro
   `logyxc tokens`, poi parser → AST → backend, a fasi.
4. **Scelte di design** (sopra) da discutere quando il nucleo è completo.

Ogni tappa segue il ciclo già rodato: implementare nei tre backend, aggiungere un esempio, mantenere
verde `tests/conformance.sh`.
