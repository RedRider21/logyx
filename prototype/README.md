# Prototipo del frontend Logyx (Python)

Prima implementazione eseguibile di Logyx: **lexer → parser → interprete tree-walking**, scritta in Python
puro (nessuna dipendenza). Serve a "sentire" il linguaggio prima di scrivere il compilatore vero in Rust.

## Uso

```
cd prototype
python3 main.py ../examples/hello.logyx
python3 main.py ../examples/demo.logyx
```

Richiede solo Python 3.8+.

## Cosa esegue (v0)

Il nucleo del linguaggio:

- funzioni (`fn`), ricorsione, `return`;
- variabili e costanti (`const`), tipi opzionali ignorati a runtime (gradual typing);
- `if / else if / else`, `while`, `for ... in ...`;
- interi, float, bool, `nil`, stringhe con interpolazione `"Ciao {nome}"`;
- liste `[...]`, mappe `{k: v}`, indicizzazione `x[i]`;
- operatori aritmetici, di confronto e logici (`and`, `or`, `not`);
- builtin: `print`, `len`, `str`, `range`.

Convenzione: se il programma definisce `fn main()`, viene chiamata automaticamente al termine.

## Cosa NON esegue ancora

I costrutti web/client (`route`, `render`, `@start-client`): richiedono il runtime server/WASM, che arriverà
più avanti. Su quei file il prototipo dà un messaggio chiaro. Vedi `examples/hello_web.logyx` per la forma
prevista.

## Struttura

| File | Ruolo |
| --- | --- |
| `logyx/lexer.py` | Sorgente → token (gestisce commenti e interpolazione) |
| `logyx/tokens.py` | Tipi di token e parole chiave |
| `logyx/parser.py` | Token → AST (discesa ricorsiva) |
| `logyx/nodes.py` | Nodi dell'AST |
| `logyx/interpreter.py` | Esecuzione dell'AST (ambiti, funzioni, builtin) |
| `main.py` | Punto d'ingresso: esegue un file `.logyx` |

## Prossimi passi del prototipo

- Precedenze complete e messaggi d'errore ancora più precisi.
- Moduli / import, gestione degli errori nel linguaggio.
- Un primo abbozzo del template `render` (solo lato server) per avvicinarsi al modello web.
