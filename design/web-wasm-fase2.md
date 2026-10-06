# Mini-design — Web, Fase 2: server nativo (route + render)

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-05

Dopo la [Fase 0](web-wasm.md) (funzioni numeriche → WASM) e la [Fase 1](web-wasm-fase1.md) (stringhe
oltre il confine + pagina interattiva), la Fase 2 porta nel **compilatore nativo** il lato **server**
del web, che finora esiste solo nel prototipo Python.

## Punto di partenza (cosa c'è già)

- **Sintassi web** (definita nel prototipo): `route "/perc" { <stmt> ... render <template> }`, dove il
  template è HTML con interpolazioni `{expr}` (HTML-escaped) e, al suo interno, isole client
  `@start-client … @end-client`. Esempi: `examples/web_demo.logyx` (solo server),
  `examples/hello_web.logyx` (server + isola client).
- **Prototipo Python**: `python main.py render <file> <path>` stampa l'HTML di una route;
  `python main.py serve <file> [porta]` avvia un server HTTP (`http.server`) che rende le route;
  `client.py` compila le isole `@start-client` in JavaScript.
- **Compilatore nativo `logyxc`**: **non** conosce `route`/`render`/`@…` (il lexer li rifiuta). Tutto
  da costruire.

## Obiettivo e crate

Un sorgente con `route`/`render` deve compilare a un **binario server nativo** che serve l'HTML (e, in
prospettiva, il `.wasm` delle isole client). Crate HTTP: **`tiny_http`** — minimale, sincrona, poche
dipendenze, perfetta per un PoC; si aggancia col meccanismo crate già pronto (build ibrido → progetto
`cargo`). Niente framework pesanti (axum/tokio) in v0.

## Sotto-fasi

- **Fase 2a — render HTML server-side nativo. FATTA.** Il compilatore impara `route`/`render`/template e
  genera l'HTML **esattamente come il prototipo** (interpolazione `{expr}` type-directed + HTML-escaping).
  Comando `logyxc render <file> <path>` → stampa l'HTML (stesso output di `python main.py render`).
  Niente server HTTP ancora; le isole `@start-client` e i costrutti `{for …}`/`{if …}` nel template danno
  un errore chiaro (fasi successive). Conforme al prototipo: caso `render web_demo /` nella suite
  (`tests/conformance.sh`, ora 39). Implementazione: lexer (cattura template), AST
  (`Item::Route`/`Stmt::Render`), parser, codegen (`__route_N()` + `main` dispatcher sul path,
  helper `__html_escape`/`__disp_float`).
- **Fase 2b — server HTTP. FATTA.** `logyxc build-server <file>` compila un server `tiny_http` (senza
  avviarlo, stampa il path del binario); `logyxc serve <file> [porta]` lo compila e lo avvia in foreground
  (default 8080). Il server fa match sul path (`__render_path`) → render della route → risposta
  `text/html; charset=utf-8`, 404 se la route non esiste. Il binario prende la porta come argomento.
  Smoke test: `tests/server_smoke.sh` (avvia, confronta il body HTTP con `render`, verifica Content-Type
  e 404). *Resta da servire gli asset statici — il `.wasm`/`.js` del client — insieme alle route (Fase 2c).*
- **Fase 2c — isole client servite.** Le isole `@start-client` del template si compilano a WASM (ABI
  Fase 1) + collante JS, iniettati nella pagina resa dal server. Qui si ricongiungono le due metà del
  "un solo sorgente". (Può slittare alla Fase 3 se il modello integrato lo assorbe.)

## Nodi tecnici (Fase 2a)

1. **Lexer**: riconoscere `route`; dopo `render`, catturare il **template grezzo** bilanciando i tag
   (come il lexer del prototipo, `_template`), in un token `Template(raw)`. Gestire `@start-client …
   @end-client` come marcatori (in 2a: errore se presenti).
2. **AST**: `Item::Route { path, body }`, `Stmt`/`Expr` per `Render { template }` (il template resta
   grezzo; l'interpolazione si risolve in codegen).
3. **Parser**: `route "<path>" <block>`; dentro, gli stessi statement del nucleo più `render <template>`.
4. **Codegen**: ogni route → `fn __route_<slug>() -> String { <stmt server>; <html> }`. Il template si
   spezza in segmenti di testo letterale e interpolazioni `{expr}`: il testo si emette verbatim, ogni
   `{expr}` come `__html_escape(&format!("{}", <expr>))`. Un `main` di `render` risolve il path → chiama
   la route giusta e stampa.
5. **HTML-escaping**: helper `__html_escape` (`& < > " '`) **identico** fra prototipo e compilatore, per
   la conformità.

## Conformità e test

- `python main.py render web_demo.logyx /` **==** `logyxc render web_demo.logyx /` (nuovo caso in una
  suite web, o estensione di `conformance.sh`).
- Il server (2b) si verifica con uno smoke test (avvio, `curl`/richiesta, confronto del body), stile
  `tests/wasm_smoke.sh`.

## Fuori scope (fasi successive)

- Route con parametri/variabili di path, query string, metodi diversi da GET.
- Stato server, sessioni, DB.
- Modello server-driven con diff del DOM via WebSocket (Fase 3).
