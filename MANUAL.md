# Manuale di Logyx

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Aggiornato:** 2026-09-28

Manuale di riferimento del linguaggio **Logyx** e delle sue funzioni, aggiornato via via che il progetto
cresce. Descrive ciò che il prototipo esegue davvero oggi; le parti non ancora fatte sono elencate in fondo.

Legenda stato: ✅ implementato · 🟡 parziale · ⏳ previsto.

---

## 1. Eseguire un programma

Dal prototipo Python (cartella `prototype/`, serve solo Python 3.8+):

| Comando | Cosa fa |
| --- | --- |
| `python3 main.py <file>.logyx` | Esegue il programma (chiama `main()` se presente) |
| `python3 main.py render <file> <percorso>` | Stampa l'HTML reso da una route |
| `python3 main.py serve <file> [porta]` | Avvia un server HTTP |
| `python3 main.py build <file>` | Transpila un sottoinsieme tipizzato in Rust (compila se c'è `rustc`) |

## 2. Sintassi di base ✅

- **Non posizionale**: i blocchi usano le graffe `{ }`; l'indentazione è solo estetica.
- **Punto e virgola opzionale**: fine riga o `;`.
- **Commenti**: `//` a riga singola, `/* ... */` a blocco.
- **Poche parentesi tonde**: nei costrutti di controllo NON si usano `()` attorno alla condizione.

```
if x > 0 { print("positivo") }      // non:  if (x > 0)
```

## 3. Tipi e valori ✅

| Tipo | Esempi | Note |
| --- | --- | --- |
| `int` | `42`, `-7` | interi |
| `float` | `3.14` | virgola mobile |
| `bool` | `true`, `false` | |
| `string` | `"ciao"` | con interpolazione `"{...}"` |
| `nil` | `nil` | assenza di valore |
| lista `[T]` | `[1, 2, 3]` | vedi §7 |
| mappa `{K: V}` | `{"nome": "Ada"}` | vedi §7 |

I tipi sono **opzionali** (gradual typing): si annotano dove servono, con `nome: tipo`.

## 4. Variabili, costanti, operatori ✅

```
x = 10                 // dichiarazione + assegnazione
nome: string = "Ada"   // con tipo esplicito
const PI = 3.14159     // costante (non riassegnabile)
```

- Aritmetici: `+  -  *  /  %` — fra interi `/` è divisione intera troncata verso zero (come Rust)
- Confronto: `==  !=  <  <=  >  >=`
- Logici: `and  or  not`
- `+` fra stringhe (o stringa e numero) concatena.
- Assegnazione composta: `+=  -=  *=  /=  %=` — per esempio `x += 1` equivale a `x = x + 1`.

## 5. Controllo di flusso ✅

```
if x > 0 { ... } else if x == 0 { ... } else { ... }
while x > 0 { x = x - 1 }
for n in numeri { print(n) }
```

Dentro `while` e `for` si possono usare `break` (esce dal ciclo) e `continue` (passa all'iterazione
successiva).

Il costrutto `match` sceglie un ramo **per valore** (confronto di uguaglianza); `else` è il ramo di
default, opzionale:

```
match n {
    0 { print("zero") }
    1 { print("uno") }
    else { print("altro") }
}
```

(Il `match` con i rami `ok`/`err` è invece la gestione degli errori, vedi §6c.)

## 6. Funzioni ✅

```
fn saluta(nome: string) -> string {
    return "Ciao " + nome
}

fn doppio(n) { return n * 2 }   // tipi opzionali

fn main() { print(saluta("mondo")) }
```

Se esiste `main()`, viene chiamata automaticamente.

## 6b. Moduli e import ✅

`import "file.logyx"` porta nel programma le definizioni (funzioni) di un altro file. Il percorso è
relativo al file che importa; l'estensione `.logyx` può essere omessa.

```
// lib_math.logyx
fn quadrato(n) { return n * n }

// main.logyx
import "lib_math.logyx"
fn main() { print(quadrato(9)) }     // 81
```

- Ogni file è caricato **una sola volta** (import ripetuti o ciclici sono gestiti).
- Un nome di funzione definito in più moduli dà un errore chiaro (niente collisioni silenziose).
- Vale sia per l'interprete sia per la compilazione nativa (`build`): l'inferenza dei tipi funziona
  anche fra le funzioni importate. Vedi `examples/use_import.logyx` e `examples/lib_math.logyx`.

## 6c. Gestione degli errori ✅

Gli errori sono **valori** (modello Result), non eccezioni. Una funzione che può fallire lo dichiara con
`-> T | error` (o lo si lascia dedurre): restituisce un valore di tipo `T` **oppure** un errore.

| Costrutto | Effetto |
| --- | --- |
| `fail "messaggio"` | Esce dalla funzione restituendo un errore con quel messaggio |
| `espr?` | Se `espr` è un errore, lo propaga (esce dalla funzione); altrimenti dà il valore |
| `match espr { ok v { … } err e { … } }` | Gestisce i due casi: `v` è il valore, `e` è il messaggio d'errore |

```
fn dividi(a, b) -> int | error {
    if b == 0 { fail "divisione per zero" }
    return a / b
}

fn meta(x) -> int | error {
    return dividi(x, 2)?          // propaga se dividi fallisce
}

fn main() {
    match dividi(10, 0) {
        ok v  { print("risultato = {v}") }
        err e { print("errore: {e}") }
    }
}
```

Vale sia per l'interprete sia per `build`: in Rust `-> T | error` diventa `Result<T, String>`, `fail`
diventa `return Err(...)`, `?` è l'omonimo operatore di Rust e `match ok/err` diventa `match Ok/Err`.
Il tipo fallibile è **dedotto** anche senza annotazione, se il corpo usa `fail` o `?`. Vedi
`examples/errori.logyx`.

## 6d. Record (tipi con campi) ✅

Un `record` è un tipo con **campi nominati** (come una struct).

```
record Persona {
    nome: string,
    eta: int
}

fn descrivi(p: Persona) -> string {
    return p.nome + " ha " + str(p.eta) + " anni"
}

fn main() {
    ada = Persona("Ada", 36)     // costruzione posizionale
    print(descrivi(ada))
    print(ada.nome)              // accesso ai campi: Ada
}
```

- **Definizione:** `record Nome { campo: tipo, ... }` — i campi hanno tipi espliciti.
- **Costruzione:** `Nome(v1, v2, ...)`, nell'ordine dei campi.
- **Accesso ai campi:** `valore.campo`.
- I record si possono passare alle funzioni e restituire. Vale per l'interprete e per `build`: in Rust
  diventano `struct` con `#[derive(Clone, PartialEq)]`. Vedi `examples/native_record.logyx`.
- I record possono stare in **liste e mappe**: `[Persona("Ada", 36), ...]`, `{"k": Persona(...)}`;
  si iterano, indicizzano e se ne accede ai campi (`gente[0].nome`, `for p in gente { p.eta }`). Vedi
  `examples/native_recordlist.logyx`.

## 7. Collezioni 🟡

Liste e mappe: letterali, indicizzazione, `len`, iterazione con `for`.

```
xs = [3, 1, 2]
xs[0]            // 3
len(xs)          // 3
for x in xs { ... }

m = {"nome": "Ada"}
m["nome"]            // "Ada"
has(m, "nome")       // true
```

Mutazioni: `push(lista, x)` aggiunge `x` in coda, `remove(lista, i)` toglie l'elemento all'indice `i`,
`sort(lista)` ordina in place. Altre operazioni: `sum(lista)`, `contains(lista, x)`, `has(mappa, chiave)`.
Non ancora: `map`/`filter`/`reduce`, slicing, `keys`/`values` e iterazione sui valori di mappa. ⏳

## 8. Stringhe e interpolazione ✅

```
nome = "Logyx"
print("Ciao da {nome}!")     // Ciao da Logyx!
```

## 9. Funzioni builtin ✅

| Funzione | Descrizione |
| --- | --- |
| `print(x)` | Stampa `x` (con conversione leggibile) |
| `len(x)` | Lunghezza di lista, mappa o stringa |
| `str(x)` | Converte in stringa |
| `range(n)` / `range(a, b)` | Lista `0 .. n-1`, oppure `a .. b-1` |
| `push(lista, x)` | Aggiunge `x` in coda alla lista (la muta); usala come istruzione |
| `remove(lista, i)` | Toglie l'elemento all'indice `i` (muta la lista); usala come istruzione |
| `abs(x)` | Valore assoluto di `x` (int o float) |
| `min(a, b)` / `max(a, b)` | Il minore / il maggiore fra due numeri dello stesso tipo |
| `upper(s)` / `lower(s)` | La stringa `s` in maiuscolo / minuscolo |
| `contains(lista, x)` | `true` se la lista contiene il valore `x` |
| `trim(s)` | La stringa `s` senza gli spazi iniziali e finali |
| `has(mappa, chiave)` | `true` se la mappa contiene la chiave |
| `index_of(s, sub)` | Posizione (ASCII) della prima occorrenza di `sub` in `s`, oppure -1 |
| `substring(s, a, b)` | Sottostringa (ASCII) da `a` incluso a `b` escluso |
| `split(s, sep)` | Divide `s` sul separatore `sep` → lista di stringhe |
| `join(lista, sep)` | Unisce una lista di stringhe con `sep` → stringa |
| `replace(s, da, a)` | Sostituisce tutte le occorrenze di `da` con `a` in `s` |
| `starts_with(s, p)` / `ends_with(s, p)` | `true` se `s` inizia / finisce con `p` |
| `map(lista, f)` | Nuova lista con la funzione `f` applicata a ogni elemento |
| `filter(lista, p)` | Gli elementi della lista per cui la funzione `p` è vera |
| `reduce(lista, init, f)` | Accumula da `init` applicando `f(acc, elemento)` |
| `to_json(x)` | Serializza `x` (int, float, bool, string, record, lista o mappa) in una stringa JSON |
| `from_json(testo, Record)` | Deserializza il JSON `testo` in un record (usa la crate Rust `serde`) |
| `sha256(s)` | Hash SHA-256 di `s` in esadecimale (usa la crate Rust `sha2`) |
| `now()` | Timestamp Unix corrente in secondi (`int`) |
| `format_date(ts)` | Formatta il timestamp Unix `ts` come data UTC `YYYY-MM-DD HH:MM:SS` (crate `chrono`) |

In `map`/`filter`/`reduce` la funzione è il **nome di una funzione definita con `fn`** (non una lambda
inline, per ora). Esempi: `examples/native_mapfilter.logyx`, `examples/native_reduce.logyx`.

`to_json` serializza **int**, **float**, **bool**, **string**, **record** (anche annidati) e **liste**,
in formato compatto; le stringhe sono "escaped". I float finiti e interi escono con il decimale (`3.0`);
le liste possono contenere qualunque tipo supportato, record e liste annidate inclusi — il tipo
dell'elemento si deduce da una lista letterale o da una variabile a cui è stata assegnata una lista
letterale (`numeri = [1,2,3]; to_json(numeri)`). Anche le **mappe** sono serializzabili (v0: solo
chiavi `string`); le chiavi escono **ordinate** così l'output è deterministico
(`prezzi = {"pane":2,"latte":1}; to_json(prezzi)` → `{"latte":1,"pane":2}`). `from_json(testo, Record)` fa l'inverso: deserializza
una stringa JSON in un record (campi scalari/string), usando la crate `serde`. Vedi
`examples/native_json.logyx` e `examples/native_fromjson.logyx`.

Nota: nelle **stringhe letterali** i caratteri `{` e `}` avviano l'interpolazione; per inserirli alla
lettera (es. in un JSON scritto a mano) vanno escapati con `\{` e `\}`.

### Librerie esterne (crates Rust)

Alcuni builtin usano **librerie dell'ecosistema Rust** "sotto il cofano": `sha256` (crate `sha2`) e
`from_json` (crate `serde`). Quando un programma usa una dipendenza esterna, il comando `build` **passa
automaticamente da `rustc` a `cargo`** (build ibrido): crea al volo un piccolo progetto `cargo`, scarica e
compila le crate, ed esegue. Senza dipendenze resta `rustc` diretto (veloce). Vedi
`examples/native_crate.logyx`.

#### Importare una crate qualsiasi (`use rust` / `extern rust`)

Oltre ai builtin, puoi agganciare **qualunque crate** dichiarando la dipendenza e scrivendo una funzione
`extern rust` con un piccolo corpo Rust che usa la crate:

```
use rust "hex" = "0.4"

extern rust fn to_hex(s: string) -> string = "hex::encode(s.as_bytes())"

fn main() {
    print(to_hex("ciao"))        // 6369616f
}
```

- `use rust "<crate>" = "<versione>"` dichiara la dipendenza (finisce nel `Cargo.toml`).
- `extern rust fn nome(p: tipo, ...) -> tipo = "<espressione Rust>"` definisce una funzione il cui corpo
  è codice Rust (i parametri sono i tipi Logyx mappati: `string`→`String`, `int`→`i64`, …). Il corpo
  controlla le conversioni verso la crate (come un binding FFI): sei tu a scrivere il "glue".
- Le funzioni `extern` girano **solo con `build`** (il nativo), non nell'interprete. Vedi
  `examples/native_extern.logyx`.

#### Compilare a WebAssembly (web → WASM)

Primo passo verso la visione "un sorgente, server + client": il comando `build-wasm` compila le funzioni
con firma fatta di `int`/`float`/`bool`/`string` a **WebAssembly** e genera una pagina HTML interattiva
che le invoca dal browser.

```
logyxc build-wasm examples/native_wasm.logyx     # funzioni numeriche
logyxc build-wasm examples/native_webstr.logyx   # funzioni su stringhe
# → <file>.wasm + <file>.html
# servi la cartella (python3 -m http.server) e apri la pagina
```

Nella pagina ogni funzione esportata ha i suoi campi (numerici o di testo) e un pulsante "esegui": è
Logyx che gira nel browser come WASM.

- **Fase 0 — numeri** (`int`/`float`/`bool`): esportati come funzioni "C", chiamati da JS con `Number`.
- **Fase 1 — stringhe**: le stringhe attraversano il confine JS↔WASM tramite la **memoria lineare** del
  modulo (il modulo esporta `__logyx_alloc`/`__logyx_free`; i parametri stringa arrivano come coppia
  `ptr, len`, i ritorni stringa come puntatore a un blocco con prefisso di lunghezza). Niente
  `wasm-bindgen`: solo il target `wasm32-unknown-unknown`. L'interattività (eventi) vive nel collante JS
  generato.

Vedi `design/web-wasm.md`, `design/web-wasm-fase1.md` e gli esempi `native_wasm`/`native_webstr`.
DOM dal codice Logyx e server HTTP sono le fasi successive.

#### Pagine server-side: `route` e `render` (web → server, Fase 2a)

Il lato **server** del web: una `route` definisce il gestore di un percorso; al suo interno si calcolano
valori (codice che gira sul server, compilato a nativo) e si rende un template HTML con `render`. Nel
template, `{espressione}` interpola un valore, **con HTML-escaping automatico** (`& < > "`).

```
fn saluto(nome) { return "Ciao, " + nome + "!" }

route "/" {
    titolo = saluto("mondo")
    render <html>
      <body><h1>{titolo}</h1></body>
    </html>
}
```

Il comando `logyxc render <file> <percorso>` compila il file a un binario server nativo e stampa l'HTML
reso per quel percorso (stesso output del prototipo `python main.py render`). Un file con `route` non
deve definire `fn main` (il main è il server). Esempio: `examples/web_demo.logyx`.

Per servire le route via HTTP (Fase 2b, crate `tiny_http`):

```
logyxc serve <file> [porta]       # compila e avvia il server (default porta 8080)
logyxc build-server <file>        # compila soltanto, stampa il path del binario
```

Il server risponde a ogni percorso con l'HTML della route corrispondente (`text/html; charset=utf-8`),
o `404` se la route non esiste. Il binario accetta la porta come argomento.

Non ancora supportati dal compilatore nativo (fasi successive): isole client `@start-client …
@end-client` e i relativi asset, costrutti `{for …}`/`{if …}` nel template. Vedi `design/web-wasm-fase2.md`.
| `pow(base, esp)` | `base` elevato a `esp` (interi, `esp >= 0`) |
| `floor(x)` / `ceil(x)` | Arrotonda verso il basso / verso l'alto (ritorna un intero) |
| `sqrt(x)` | Radice quadrata (ritorna un float) |
| `sum(lista)` | Somma degli elementi (lista di interi) |
| `sort(lista)` | Ordina la lista in ordine crescente (la muta); usala come istruzione |

## 10. Web lato server ✅

- `route "/percorso" { ... }` definisce un endpoint; il corpo gira sul server.
- `render <html>...</html>` produce HTML con:
  - **interpolazione** `{espressione}` e **escaping automatico** dei valori;
  - **blocchi di controllo**: `{ for x in xs { ... } }` e `{ if cond { ... } else { ... } }`.

```
route "/todo" {
    voci = ["pane", "latte"]
    render <html><body>
      <ul>{ for v in voci { <li>{v}</li> } }</ul>
    </body></html>
}
```

Nota: i tag void vanno auto-chiusi (`<br/>`). 🟡

## 11. Isole client (→ JavaScript) ✅

Il codice fra `@start-client` e `@end-client` è compilato a JavaScript ed emesso in uno `<script>`.

DSL client v0:

| Costrutto | Effetto |
| --- | --- |
| `nome = <espr>` | stato / assegnazione |
| `on "<evento>" of "<sel>" { ... }` | gestore di evento (`addEventListener`) |
| `set text of "<sel>" to <espr>` | aggiorna `textContent` |
| `set html of "<sel>" to <espr>` | aggiorna `innerHTML` |
| `if` / `for` / `while` | controllo di flusso (tradotto in JS) |

Un piccolo runtime rende disponibili anche nel client `range`, `len`, `str`, `print`.
Target WASM: previsto. ⏳

## 12. Compilazione nativa (transpiler Rust) 🟡

`build` traduce un **sottoinsieme tipizzato** in Rust, che `rustc` compila a eseguibile nativo.

- Supporta: funzioni con tipi (`int`/`float`/`bool`/`string`), aritmetica, confronti, logica,
  concatenazione di stringhe con `+`, `if`/`else`, `while`, `for x in range(n)` o `range(a, b)`, `return`,
  `print`, ricorsione. Gli interi sono `i64`; `/` è divisione intera.
- **Tipi di ritorno e dei parametri possono essere omessi**: vengono dedotti dall'uso —
  aritmetica → `int`/`float`, concatenazione con `+` → `string`, `not`/`and`/`or` o uso come condizione
  → `bool`, confronti e chiamate propagano il tipo dell'altro lato. L'annotazione resta possibile e serve
  solo quando l'uso non basta (es. un parametro che compare unicamente in un'interpolazione).
- **Liste** (di `int`/`float`/`bool` **o `string`**): letterale `[...]` → `Vec<T>`, indicizzazione
  `xs[i]`, `len(xs)`, e `for x in xs`. Usale come variabili locali. Vedi `examples/native_list.logyx`
  e `examples/native_strlist.logyx`.
- **Mappe**: letterale `{k: v}` → `HashMap<K, V>`, accesso `m[k]`, `len(m)`, `has(m, k)`. Chiavi e
  valori possono essere scalari **o stringhe**. Vedi `examples/native_map.logyx`.
- Non ancora: iterazione su mappa (`keys`/`values`), collezioni come parametro/ritorno di
  funzione, `route`/`render`, codice dinamico senza tipi. In quei casi dà un errore chiaro.

```
fn fib(n) {                       // nessuna annotazione
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}
```
→ genera `fn fib(n: i64) -> i64 { ... }` in Rust: `n` è dedotto `int` dall'uso (`n < 2`, `n - 1`),
il ritorno `int` dal corpo. Vedi `examples/native_infer.logyx`.

## 13. Cosa non c'è ancora ⏳

- Mutazioni/metodi su liste, mappe e stringhe; libreria standard.
- Moduli / import; gestione degli errori nel linguaggio.
- Lettura input nella DSL client (`value of ...`); target WASM del client.
- Transpiler Rust completo (oltre il sottoinsieme tipizzato).
