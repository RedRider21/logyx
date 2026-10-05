# Mini-design — Web → WebAssembly (client) e server

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-04

Obiettivo: realizzare la visione originale di Logyx (`DESIGN.md`) — **un solo sorgente che gira nativo sul
server e come WebAssembly nel browser**. È il fronte più grande del progetto: qui si definisce la rotta,
non si implementa.

## La visione (richiamo)

- Server e client da **un sorgente**, con il confine marcato dalle isole `@start-client … @end-client`.
- Modello di default **server-driven** (stato sul server, il client riceve i diff del DOM); seconda marcia
  RPC/idratazione per interazioni istantanee.
- Il client è **WASM** (nel browser non si installa nulla); il server è **nativo**.

## Stato attuale (onesto)

- **Nucleo nativo**: solido e maturo (compilatore `logyxc`, 36 builtin, record, JSON, crate Rust).
- **Web**: esiste solo come **abbozzo nel vecchio prototipo Python** — `route`/`render` lato server e
  isole `@start-client` compilate a **JavaScript** (`prototype/logyx/client.py`). Il **compilatore nativo
  `logyxc` NON** gestisce i costrutti web (`@`/route/render danno errore).
- Quindi: per il web, oggi non c'è nulla nel compilatore. Va costruito.

## I tre pezzi da costruire

1. **Client → WASM.** Compilare (un sottoinsieme di) Logyx a `wasm32-unknown-unknown` e farlo girare nel
   browser. Serve la toolchain WASM di Rust (`rustup target add wasm32-unknown-unknown`) e il ponte
   JS↔WASM (crate `wasm-bindgen`, e `web-sys` per il DOM). Il backend genera un crate `cdylib` con le
   funzioni esportate `#[wasm_bindgen]`.
2. **Server → nativo + HTTP.** Routing e HTTP con una crate (es. `axum`/`tiny_http`), usando il
   meccanismo crate già pronto. Il server serve l'HTML + il modulo WASM e, nel modello server-driven,
   i diff del DOM.
3. **Il collante.** Dividere un sorgente nelle due parti (isole client), definire il **trasporto** oltre
   il confine (per il server-driven: WebSocket con i diff; per l'RPC: chiamate), e la serializzazione
   (abbiamo già JSON).

## Nodi tecnici principali

- **Toolchain WASM**: va verificata la disponibilità di `wasm32-unknown-unknown` e di `wasm-bindgen`/
  `wasm-pack` nell'ambiente (prerequisito della Fase 0).
- **Ownership e DOM**: il DOM è gestito da `web-sys` con handle; il nostro modello "clone uniforme" va
  adattato (gli handle del DOM non si clonano a cuor leggero).
- **Dimensione del WASM**: va tenuta sotto controllo (`opt-level = "z"`, `wasm-opt`).
- **Due artefatti da un sorgente**: oggi `build` produce un eseguibile; il web richiede di produrre
  (almeno) un `.wasm` + un `.html`, e in prospettiva anche il binario server.

## Percorso a fasi (proposta)

Si parte dal **client (WASM)**, la parte distintiva; il server può poi appoggiarsi a crate HTTP mature.

- **Fase 0 — Proof of concept Logyx→WASM.** Un comando `logyxc build-wasm <file>` che prende funzioni del
  **nucleo** (niente DOM), le compila a WASM con `wasm-bindgen`, e genera una paginetta HTML che importa
  il modulo e chiama una funzione (es. `fn raddoppia(n: int) -> int` invocata da un bottone). Dimostra
  che un sorgente Logyx gira nel browser. *Nessun DOM dal lato Logyx ancora.*
- **Fase 1 — stringhe oltre il confine + interattività. FATTA** (vedi [`web-wasm-fase1.md`](web-wasm-fase1.md)).
  Scelta effettiva: **niente `wasm-bindgen`** — le stringhe attraversano il confine JS↔WASM tramite la
  **memoria lineare** (helper `__logyx_alloc`/`__logyx_free`, parametri stringa come coppia `ptr,len`,
  ritorni stringa come puntatore a `[len u32][byte]`). La pagina generata è interattiva (campi di testo +
  eventi nel collante JS). Firme ammesse: `int|float|bool|string`. Esempio `examples/native_webstr.logyx`,
  verificato con Node. *Manipolare il DOM dal codice Logyx* (`set text of …`, `on "click" …` via import
  host/`web-sys`) resta un fronte successivo.
- **Fase 2 — Server HTTP.** `route "/" { render <html>… }` compilato a un server nativo (crate HTTP) che
  serve HTML + WASM. Riuso del parser template del prototipo, portato nel compilatore. **Design dedicato:**
  [`web-wasm-fase2.md`](web-wasm-fase2.md) — sotto-fasi 2a (render HTML nativo), 2b (server `tiny_http`),
  2c (isole client servite).
- **Fase 3 — Modello integrato.** Isole `@start-client` che dividono il sorgente; server-driven con
  WebSocket + diff del DOM (serializzati in JSON); poi RPC/idratazione.

## Decisioni da prendere insieme (prima della Fase 1+)

- **Toolchain**: `wasm-bindgen` + HTML a mano (leggero) oppure `trunk`/`wasm-pack` (più integrati).
- **Modello client**: ricominciare dalle primitive del prototipo (`on`/`set text`) o esporre `web-sys`
  più direttamente?
- **Confine**: confermare `@start-client` come marcatore e il trasporto (WebSocket per i diff).
- **Conformità**: il lato WASM/DOM **non** è verificabile con la suite attuale (serve un browser headless).
  Serve una strategia di test a parte per il client (es. `wasm-bindgen-test`, o uno smoke test manuale).

## Prerequisiti verificati (2026-10-04)

- `wasm32-unknown-unknown`: **non installato** ma **disponibile** → `rustup target add wasm32-unknown-unknown`.
- `wasm-bindgen` / `wasm-pack` / `trunk`: **assenti**. `wasm-bindgen-cli` si installa con
  `cargo install wasm-bindgen-cli` (compila, richiede tempo e rete).

**Scorciatoia per la Fase 0 (niente wasm-bindgen):** le funzioni con argomenti/ritorni **numerici**
(`i64`/`f64`/`bool`) si esportano in WASM come funzioni "C" (`#[no_mangle] pub extern "C"`) e si chiamano
da JS con l'API standard `WebAssembly.instantiate(...).exports.nome(...)`. Quindi la Fase 0 è fattibile
col **solo target wasm32** (installabile), senza `wasm-bindgen`. Le **stringhe** (passaggio di memoria)
richiederanno `wasm-bindgen` e arriveranno dopo.

## Primo passo concreto (Fase 0)

1. `rustup target add wasm32-unknown-unknown`.
2. Un comando `logyxc build-wasm <file>`: genera un crate `cdylib`, esporta le `fn` del nucleo con
   argomenti/ritorni numerici come `#[no_mangle] pub extern "C"`, compila a `wasm32`, e produce
   `<file>.wasm` + un `<file>.html` che carica il modulo e chiama una funzione (es. un bottone che
   calcola `raddoppia(21)` → 42).
3. Aprire la pagina nel browser per vedere Logyx girare come WebAssembly.

È un risultato tangibile, a basso rischio, e apre la strada a DOM/eventi (Fase 1) e al server (Fase 2).
