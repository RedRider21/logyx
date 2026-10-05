# Mini-design — Web → WASM, Fase 1: stringhe e interattività

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-05

Segue la [Fase 0](web-wasm.md) (funzioni a firma numerica compilate a WASM e chiamate dal browser).
Obiettivo della Fase 1: **far passare le stringhe** oltre il confine JS↔WASM e rendere la pagina demo
**interattiva** (campi di testo + eventi), così da coprire i casi reali `string -> string`,
`string -> int`, `int -> string`, ecc.

## Scelta di fondo: niente `wasm-bindgen` (ancora)

Il design originale ipotizzava `wasm-bindgen` + `web-sys` per le stringhe e il DOM. Si rimanda:
`wasm-bindgen` aggiunge una dipendenza il cui `-cli` va installato e **tenuto allineato di versione**
con la crate, allunga la catena di build e porta glue JS non nostro.

In Fase 1 restiamo sul **solo target `wasm32-unknown-unknown`** (già usato in Fase 0) e passiamo le
stringhe tramite la **memoria lineare** del modulo, con un ABI minimale scritto da noi. Il **DOM e gli
eventi restano nel collante JS** generato (un `<input>` e un bottone che chiama la funzione WASM e
scrive il risultato in un nodo): è esattamente l'interattività "leggi testo → calcola → mostra testo"
senza dover manipolare il DOM dal lato Rust. Manipolare il DOM *dal codice Logyx* (handle `web-sys`)
resta un fronte successivo, da aprire solo se servirà davvero.

## ABI delle stringhe (memoria lineare)

Il modulo esporta sempre, quando compare almeno una stringa, due helper di memoria:

```
__logyx_alloc(len: i32) -> i32      // riserva len byte, ritorna il puntatore
__logyx_free(ptr: i32, len: i32)    // libera un blocco di len byte
```

- **JS → WASM (parametro stringa).** JS codifica in UTF-8, chiama `__logyx_alloc(len)`, scrive i byte
  all'offset ottenuto e passa alla funzione la coppia `(ptr: i32, len: i32)`. Il **wrapper WASM
  ricostruisce la `String` e libera subito quel blocco di input** (JS non lo rilibera).
- **WASM → JS (ritorno stringa).** Il wrapper alloca un blocco `[len: u32 LE][byte UTF-8…]` e ritorna
  `ptr: i32`. JS legge i 4 byte di lunghezza, poi i `len` byte dei dati, e infine chiama
  `__logyx_free(ptr, 4 + len)`.

La `memory` del modulo è esportata di default dai cdylib `wasm32-unknown-unknown`, quindi JS vi accede
come `instance.exports.memory`. Dopo ogni `__logyx_alloc` la memoria può crescere: JS riprende sempre
una *view* fresca (`new Uint8Array(ex.memory.buffer, …)`) dopo l'allocazione e dopo la chiamata.

Allocazione e deallocazione usano la **stessa `Layout { size, align: 1 }`** (il `size` che JS passa a
`__logyx_free` coincide con quello allocato), così il blocco è liberato correttamente.

## Firme supportate

`wasm_exports` (ex `numeric_exports`) accetta ora i tipi di parametro e di ritorno
`int | float | bool | string`. Mappatura verso WASM:

| Tipo Logyx | Parametro WASM        | Ritorno WASM         | In JS                     |
|------------|-----------------------|----------------------|---------------------------|
| `int`      | `i32` (poi `as i64`)  | `i32`                | `Number`                  |
| `float`    | `f64`                 | `f64`                | `Number`                  |
| `bool`     | `i32` (`!= 0`)        | `i32` (`as i32`)     | `Boolean` (`r !== 0`)     |
| `string`   | `i32 ptr, i32 len`    | `i32 ptr` (len-pref) | `String` (TextEncoder/Decoder) |

Un parametro stringa occupa **due** argomenti WASM (ptr, len); i numerici uno solo, come in Fase 0.

## Collante HTML generato

La pagina incorpora un descrittore delle firme e un marshaller generico:

```js
const SIGS = [ {name:"saluta", params:["string"], ret:"string"}, … ];
```

Per ogni funzione genera una riga con i campi giusti (`<input type=text>` per le stringhe, `number`
per i numeri), un bottone e uno spazio per il risultato. Il marshaller `callFn(sig, rawArgs)` applica
l'ABI sopra. L'interattività (evento `click`) vive qui, nel JS: è la Fase 1 "DOM/eventi" nella sua
forma minima e onesta.

## Conformità e test

- La **logica** `string→string` ecc. è normale codice di nucleo: gli esempi hanno un `main` che stampa i
  risultati e **entrano nella suite `conformance.sh`** (interprete vs compilatore nativo). `build-wasm`
  ignora `main`, quindi un unico file serve a entrambi.
- Il **lato WASM** non è verificabile dalla suite di conformità: c'è uno **smoke test dedicato**,
  `tests/wasm_smoke.sh` (+ `tests/wasm_smoke.mjs`). Compila gli esempi del client a WASM ed esegue con
  Node il **collante JS reale** delle pagine generate (non una sua reimplementazione), con un DOM
  simulato minimale, verificando numeri e stringhe (round-trip e UTF-8 accentato). Salta con codice 77
  se mancano `node`/`cargo`/il target `wasm32`.

## Fuori scope (fasi successive)

- Manipolare il DOM *dal codice Logyx* (sintassi `on "click"…`, `set text of…`) → richiederebbe import
  host o `web-sys`.
- Liste/mappe/record oltre il confine (servirebbe serializzare, p. es. in JSON).
- Server HTTP (Fase 2) e modello integrato server-driven (Fase 3).
