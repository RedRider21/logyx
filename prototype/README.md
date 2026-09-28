# Prototipo del frontend Logyx (Python)

Prima implementazione eseguibile di Logyx: **lexer → parser → interprete tree-walking**, scritta in Python
puro (nessuna dipendenza). Serve a "sentire" il linguaggio prima di scrivere il compilatore vero in Rust.

## Uso

```
cd prototype
python3 main.py ../examples/demo.logyx                 # esegue il nucleo (chiama main())
python3 main.py render ../examples/web_demo.logyx /    # stampa l'HTML reso da una route
python3 main.py serve  ../examples/web_demo.logyx 8137 # server HTTP su 127.0.0.1:8137
```

Richiede solo Python 3.8+ (nessuna dipendenza).

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

## Web lato server (render)

- `route "/percorso" { ... }` definisce un endpoint; il corpo gira sul server.
- `render <html>...</html>` produce HTML con interpolazione `{espressione}` e **escaping automatico** dei
  valori interpolati.
- I blocchi `@start-client ... @end-client` sono resi come un segnaposto (commento HTML): il loro codice
  diventerà WASM quando ci sarà il runtime client.

Comandi `render` (stampa l'HTML) e `serve` (server HTTP) qui sopra.

## Cosa NON esegue ancora

- Nel template: blocchi di controllo (`{ for x in xs { ... } }`) — per ora solo interpolazione `{expr}`.
- Il lato client vero (isole `@start-client` compilate a WASM) e i tag void non chiusi (usare `<br/>`).

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

- Blocchi di controllo nel template (`{ for ... }`, `{ if ... }`).
- Moduli / import e gestione degli errori nel linguaggio.
- Primo abbozzo del lato client (isole `@start-client`) verso WASM.
