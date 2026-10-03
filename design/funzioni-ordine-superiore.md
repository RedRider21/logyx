# Mini-design — Funzioni di ordine superiore (map / filter / reduce)

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-03

Obiettivo: `map`, `filter`, `reduce` sulle liste, mantenendo la suite di conformità verde.

## Il nodo: closure in Rust

`map`/`filter`/`reduce` di solito prendono una **funzione** come argomento. In Rust ciò apre un fronte
complesso: le closure hanno tipi anonimi, catturano l'ambiente e si passano come generici (`impl Fn`) o
trait object (`Box<dyn Fn>`), con i relativi lifetime. Inoltre Logyx non ha ancora un **tipo funzione**.

## Scelta: due fasi

**Fase 1 (questa) — funzioni *nominate*, niente valori funzione nel linguaggio.**
`map`/`filter`/`reduce` accettano come argomento il **nome di una funzione definita con `fn`** (non una
lambda inline, non un valore funzione assegnabile). Il backend genera la chiamata diretta in Rust: così
**non serve** un tipo funzione né le closure del linguaggio.

```
fn raddoppia(n) { return n * 2 }
fn pari(n) { return n % 2 == 0 }
fn somma2(a, b) { return a + b }

fn main() {
    xs = [1, 2, 3, 4, 5]
    print("{map(xs, raddoppia)}")        // [2, 4, 6, 8, 10]
    print("{filter(xs, pari)}")          // [2, 4]
    print("totale = {reduce(xs, 0, somma2)}")  // 15
}
```

**Fase 2 (futura) — valori funzione e lambda inline.** Richiede un tipo funzione in Logyx e, nel backend,
`Box<dyn Fn(...)>`/fn-pointer. Design separato, quando servirà.

## Semantica

| Builtin | Significato | Interprete | Rust generato |
| --- | --- | --- | --- |
| `map(lista, f)` | nuova lista con `f` su ogni elemento | `[f(x) for x in lista]` | `(lista).iter().cloned().map(\|x\| f(x)).collect::<Vec<_>>()` |
| `filter(lista, p)` | elementi per cui `p` è vero | `[x for x in lista if p(x)]` | `(lista).iter().cloned().filter(\|x\| p(x.clone())).collect::<Vec<_>>()` |
| `reduce(lista, init, f)` | accumula da `init` con `f(acc, x)` | `acc=init; for x: acc=f(acc,x)` | `(lista).iter().cloned().fold(init, \|acc, x\| f(acc, x))` |

- `f`/`p` devono essere un **nome di funzione** (identificatore). Altrimenti: errore chiaro.
- Nel backend, `f` deve essere una **funzione definita con `fn`** (non un builtin come `abs`, che in Rust
  è un metodo e non una funzione libera). Gli esempi usano funzioni dell'utente.
- `.iter().cloned()` è coerente con l'opzione "clone uniforme" già adottata per le collezioni.

## Tipi

- `map`/`filter` → **lista** (il risultato è una `Vec`): non un tipo scalare, quindi nessun tipo dedotto
  per l'inferenza (usabili come variabili locali, come le altre liste).
- `reduce` → il tipo di `init` (l'accumulatore).

## Conformità

La suite confronta l'output. Nell'interprete `f`/`p` sono `LogyxFunction` chiamate con `self.call`; nel
nativo sono funzioni Rust chiamate per nome: stesso risultato. Gli esempi useranno funzioni dell'utente e
liste di scalari per restare semplici.

## Passi

1. `map` + `filter` (un giro), esempio, suite verde.
2. `reduce` (un giro), esempio, suite verde.
3. `MANUAL.md` + `docs/manuale.html` + "Riferimento" del sito + `ROADMAP.md`.
