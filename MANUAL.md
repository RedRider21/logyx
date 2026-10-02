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

## 5. Controllo di flusso ✅

```
if x > 0 { ... } else if x == 0 { ... } else { ... }
while x > 0 { x = x - 1 }
for n in numeri { print(n) }
```

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

## 7. Collezioni 🟡

Liste e mappe: letterali, indicizzazione, `len`, iterazione con `for`.

```
xs = [3, 1, 2]
xs[0]            // 3
len(xs)          // 3
for x in xs { ... }

m = {"nome": "Ada"}
m["nome"]        // "Ada"
```

Mutazioni: `push(lista, x)` aggiunge `x` in coda, `remove(lista, i)` toglie l'elemento all'indice `i`.
Non ancora: slicing, iterazione sui valori di mappa. ⏳

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
- **Liste di scalari** (`int`/`float`/`bool`): letterale `[...]` → `Vec<T>`, indicizzazione `xs[i]`,
  `len(xs)`, e `for x in xs` (elementi presi per valore). Usale come variabili locali. Vedi
  `examples/native_list.logyx`.
- **Mappe con valori scalari**: letterale `{k: v}` → `HashMap<K, V>`, accesso `m[k]`, `len(m)`.
  Le chiavi possono essere stringhe o scalari; i valori scalari. Vedi `examples/native_map.logyx`.
- Non ancora: liste/mappe di stringhe, iterazione su mappa, collezioni come parametro/ritorno di
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
