# Mini-design — Aggancio alle crates Rust

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-03

Obiettivo: usare da Logyx le librerie dell'ecosistema Rust (crates.io). È la porta principale verso le
librerie esterne (vedi `ROADMAP.md`, Piano 2.5). È il fronte più grande finora: tocca l'infrastruttura di
compilazione e introduce sintassi nuova.

## Nodo 1 — infrastruttura: da `rustc` a `cargo`

Oggi `logyxc build` genera **un solo** `.rs` e lo compila con `rustc -O file.rs`. Le crates richiedono
**`cargo`** (un `Cargo.toml` con le dipendenze). Proposta: **build ibrido**
- nessuna dipendenza dichiarata → resta `rustc` diretto (veloce, come ora: la suite non rallenta);
- almeno una dipendenza → si crea al volo un piccolo progetto `cargo` (in una cartella di build dedicata,
  gitignored) con `Cargo.toml` + `src/main.rs`, poi `cargo build --release` e si esegue il binario.

## Nodo 2 — dichiarare una dipendenza (SCELTA DI SINTASSI)

Serve un modo per dire "voglio la crate X versione Y". Alternative:

- **A)** `use rust "serde_json" = "1"` — direttiva a primo livello.
- **B)** `extern crate "serde_json" = "1"` — richiama `extern`/`crate`.
- **C)** un file manifest separato `logyx.deps` con l'elenco delle crates.

Proposta: **A** (`use rust "<crate>" = "<versione>"`), esplicita e locale al sorgente.

## Nodo 3 — usare funzioni/tipi della crate (SCELTA DI SINTASSI)

Rust non ha una ABI stabile: non si "importa e basta". Serve dichiarare **cosa** si usa, con la firma —
come si fa con la FFI C. Alternative per la v0 (minima):

- Dichiarare le **firme** delle funzioni libere che si vogliono chiamare, con i tipi Logyx mappati ai
  tipi Rust, es.:
  ```
  use rust "fastrand" = "2"
  extern rust {
      fn fastrand::u64(a: int, b: int) -> int     // chiama fastrand::u64(a..b) ecc.
  }
  ```
  Poi si chiama `fastrand_u64(...)` come una funzione normale; il backend genera la chiamata Rust reale.

Limiti v0: solo **funzioni libere** con argomenti/ritorni di tipi **scalari/string** (niente tipi/metodi
complessi della crate, niente generici). Copre già molti casi utili (random, hashing, tempo, piccole
utility). I tipi ricchi delle crate (es. i valori di `serde_json`) arrivano in una fase successiva.

## Alternativa pragmatica (più piccola e subito utile)

Invece dell'aggancio **generico** (grosso), un primo passo mirato: far usare **`serde`/`serde_json` al
compilatore "sotto il cofano"** per implementare **`from_json`** (deserializzazione nei record) e
irrobustire `to_json`. L'utente non scrive sintassi di import: è il backend che, quando serve, aggiunge
la dipendenza e genera il codice. Vantaggi: completa il JSON (molto utile), introduce il meccanismo
`cargo` in modo controllato, nessuna nuova sintassi da decidere ora.

## Due strade, da scegliere insieme

1. **Aggancio generico alle crates** (Nodi 1+2+3): potente ma grande; richiede di fissare la sintassi
   (`use rust` / `extern rust`).
2. **serde sotto il cofano → `from_json`** (alternativa pragmatica): più piccolo, completa il JSON, e
   costruisce comunque l'infrastruttura `cargo` che servirà per la strada 1.

Raccomandazione: **cominciare dalla strada 2** (infrastruttura `cargo` + `from_json` con serde), che dà
un risultato concreto subito e prepara il terreno; poi affrontare la strada 1 (aggancio generico) con la
sintassi già discussa.

## Passi (strada 2)

1. Build ibrido `rustc`/`cargo` (cartella di build gitignored), suite 30/30 invariata.
2. `from_json` con `serde_json` sotto il cofano: `from_json(testo)` → record (tipi noti dal contesto).
3. Esempio, suite, MANUAL/manuale.html/sito/ROADMAP.
