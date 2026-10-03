# Mini-design — Collezioni di stringhe

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-03

Obiettivo: permettere **liste di stringhe** (`["a", "b"]`) e **mappe con valori stringa**
(`{"k": "v"}`) nel compilatore nativo, mantenendo la suite di conformità verde. È il prerequisito
per `split` (→ lista di stringhe) e `join`.

## Il problema: `Copy` vs non-`Copy` in Rust

Oggi le liste sono `Vec<i64>`/`Vec<f64>`/`Vec<bool>`, cioè di tipi **`Copy`**. Il backend sfrutta questo:

| Operazione | Codice generato oggi | Perché funziona solo con `Copy` |
| --- | --- | --- |
| iterazione `for x in xs` | `for x in (xs).iter().copied()` | `.copied()` richiede `T: Copy` |
| indicizzazione `xs[i]` | `xs[(i) as usize]` (per valore) | muovere fuori da `Vec` è lecito solo se `Copy` |
| accesso mappa `m[k]` | `(*m.get(&k).unwrap())` (deref) | il deref copia solo se `V: Copy` |

`String` **non è `Copy`**: con quei generatori, una lista/mappa di stringhe non compila (si tenta di
*muovere* un valore fuori dalla collezione, che Rust vieta). Per questo oggi il backend le **rifiuta**
con un errore chiaro.

## Opzioni

**A — Clonare sempre (uniforme).** Generare sempre `.cloned()` al posto di `.copied()`, `xs[i].clone()`
al posto di `xs[i]`, `m.get(&k).unwrap().clone()` al posto del deref. `clone()` esiste sia per `i64`
(dove equivale a una copia, azzerata dall'ottimizzatore) sia per `String`. **Non serve conoscere il tipo
degli elementi.**
- Pro: minimale, un solo percorso di codice, corretto per int/float/bool/String; output identico →
  conformità preservata.
- Contro: `clone()` ridondante sui tipi `Copy` nel sorgente Rust (ma `rustc -O` lo elimina).

**B — Tracciare il tipo degli elementi.** Dedurre `Vec<i64>` vs `Vec<String>` e usare `.copied()` per i
`Copy` e `.cloned()`/`clone()` per `String`. Più idiomatico, zero clone inutili, ma richiede un piccolo
*type environment* per le collezioni locali (più codice, più superfici di bug).

**C — Riferimenti** (`for x in &xs` con `x: &String`). Evita i clone ma costringe a `*`/`&` ovunque negli
usi: invasivo e poco leggibile. Scartata.

## Scelta: **Opzione A** (per la v0)

È la più semplice e sicura, e preserva la conformità senza introdurre inferenza sul tipo elemento. I
clone ridondanti sui tipi `Copy` non costano nulla a runtime dopo l'ottimizzazione. In futuro si potrà
passare a **B** per il sorgente Rust più pulito, se e quando servirà.

## Modifiche concrete (transpiler Python e compilatore Rust, in parallelo)

1. **Iterazione lista**: `(xs).iter().copied()` → `(xs).iter().cloned()`.
2. **Indicizzazione lista**: `xs[(i) as usize]` → `xs[(i) as usize].clone()`.
3. **Accesso mappa**: `(*m.get(&k).unwrap())` → `m.get(&k).unwrap().clone()`.
4. **Letterale lista di stringhe**: togliere il rifiuto; `["a","b"]` → `vec!["a".to_string(), "b".to_string()]`
   (già prodotto da `expr` sulle stringhe).
5. **Mappa con valori stringa**: togliere il rifiuto; `{"k":"v"}` → `insert("k".to_string(), "v".to_string())`.

Tutto il resto resta uguale: `push`/`remove`/`contains`/`sort`/`len` funzionano già su `Vec<String>`;
`sum` resta solo per interi (documentato).

## Perché la conformità resta verde

La suite confronta l'**output d'esecuzione**, non il codice Rust. `clone()` su `i64`/`f64`/`bool` produce
lo stesso valore della copia; il `.rs` cambia ma il risultato no. I 22 esempi attuali devono restare 22/22
(da verificare col primo commit del cambiamento).

## Cosa sblocca, poi

- **`split(s, sep)` → lista di stringhe.** Rust: `s.split(sep).map(|x| x.to_string()).collect::<Vec<String>>()`.
  ⚠️ Conformità: il comportamento sui separatori vuoti o consecutivi può differire tra Python e Rust →
  scegliere esempi semplici e documentare la semantica (allineare l'interprete a quella di Rust se serve).
- **`join(lista, sep)` → stringa.** Rust: `(lista).join(sep)` su `Vec<String>`.
- Inoltre: liste/mappe di stringhe diventano utilizzabili ovunque (dati testuali, piccoli record).

## Passi

1. Applicare le 5 modifiche (Opzione A), un commit, suite 22/22 verde (solo il `.rs` cambia).
2. Aggiungere un esempio `native_strlist.logyx` (lista di stringhe: push, index, for, sort) → suite 23.
3. Implementare `split` e `join` con esempi dedicati.
4. Aggiornare `MANUAL.md`, rigenerare `docs/manuale.html`, aggiornare il "Riferimento" del sito.
