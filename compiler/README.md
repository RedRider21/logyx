# logyxc — il compilatore di Logyx (in Rust)

Fase 1 del bootstrap: il compilatore vero, scritto in **Rust**, che rimpiazzerà via via il
prototipo Python. Oggi copre lo stesso **sottoinsieme nativo** del prototipo ed è verificato
**conforme** (stesso output) sugli esempi in `../examples/`.

## Com'è fatto

| File | Ruolo |
| --- | --- |
| `src/token.rs` | Tipi di token |
| `src/lexer.rs` | Lexer (sorgente → token) |
| `src/ast.rs` | Albero sintattico |
| `src/parser.rs` | Parser a discesa ricorsiva (token → AST) |
| `src/modules.rs` | Risoluzione degli `import` |
| `src/codegen.rs` | Inferenza dei tipi e generazione del codice Rust |
| `src/main.rs` | CLI |

## Uso

```bash
cargo build --release

# ispezione
./target/release/logyxc tokens ../examples/native_fib.logyx   # token
./target/release/logyxc parse  ../examples/native_fib.logyx   # AST
./target/release/logyxc gen    ../examples/native_fib.logyx   # Rust generato

# compilazione nativa (richiede rustc nel PATH)
./target/release/logyxc build  ../examples/errori.logyx
```

## Stato e cosa manca

Copre: funzioni con inferenza dei tipi (ritorno e parametri), aritmetica, confronti, logica,
stringhe e interpolazione, `if`/`while`/`for` (range e liste), liste e mappe di scalari, moduli
(`import`), gestione errori (`fail`/`?`/`match` → `Result<T, String>`).

Non ancora (come il prototipo): costrutti web (`route`/`render`/isole client), collezioni di
stringhe, iterazione su mappa. L'orizzonte è il **self-hosting**: riscrivere il compilatore in
Logyx stesso, così da non dipendere più né da Python né da Rust.
