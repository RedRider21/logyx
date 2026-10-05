# Mini-design — JSON

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-03

Obiettivo: serializzare i valori di Logyx in JSON, valorizzando i record appena introdotti.

## Scope della v0

- **Solo serializzazione**: `to_json(x)` → stringa JSON. La **deserializzazione** (`from_json`) è
  rimandata: richiede un parser JSON scritto a mano nel compilatore (fronte a sé).
- Tipi supportati: **int**, **float**, **bool**, **string**, **record** (con campi di questi tipi, anche
  record annidati) e **liste** (di qualunque tipo supportato, record e liste annidate inclusi) — la
  serializzazione è ricorsiva.
- **float**: formato condiviso dai due backend (helper `__json_float`): i finiti interi escono con il
  decimale (`3.0`), gli altri con `format!("{}", x)` (Rust) / `repr` (Python), che per i float "puliti"
  usati negli esempi coincidono; i non finiti (NaN/∞) sono rifiutati (non sono JSON validi).
- **liste**: il tipo dell'elemento si deduce da una lista letterale o da una variabile a cui è stata
  assegnata una lista letterale (mappa `list_elem`); non da liste prodotte da `map`/`split`/ecc.
- **Esclusi per ora**: **mappe** (il backend non traccia il tipo dei valori).

## Perché serve tracciare i tipi delle variabili locali

`to_json` è **type-directed**: genera codice diverso secondo il tipo dell'argomento (un numero, una
stringa con escape, un oggetto per i record). Oggi il backend conosce i tipi dei **parametri** ma non
quelli delle **variabili locali**. Per far funzionare `to_json(ada)` con `ada` locale, si aggiunge un
piccolo **ambiente di tipi locale**: a ogni `nome = valore` si registra il tipo dedotto di `valore`
(utile anche in generale). Cambiamento isolato, da verificare con la suite (28/28 invariati).

## Serializzazione (uguale nei due backend, per conformità)

- `int` → `format!("{}", x)` · `bool` → `format!("{}", x)` (`true`/`false`)
- `string` → escape JSON (funzione comune `__json_str`): `"`, `\`, `\n`, `\t`, `\r`, e i caratteri di
  controllo come `\u00XX`.
- `record` → `{"campo1":<json>,"campo2":<json>}` nell'ordine di definizione dei campi (ricorsivo).

Formato **compatto** (niente spazi), stesso ordine dei campi: così l'output dell'interprete e del nativo
coincide esattamente. L'interprete usa la stessa identica logica di escape e lo stesso ordine.

Esempio:

```
record Persona { nome: string, eta: int }

fn main() {
    ada = Persona("Ada", 36)
    print(to_json(ada))        // {"nome":"Ada","eta":36}
}
```

## Dettagli di implementazione

- **`__json_str`**: funzione helper emessa nel preludio del `.rs` **solo se** `to_json` è usato
  (flag `uses_json`). Stessa logica replicata nell'interprete.
- **`to_json`** è un builtin value-returning di tipo `string`. In fase di generazione risolve il tipo
  dell'argomento tramite l'ambiente dei tipi (parametri + locali) e produce il serializzatore adatto;
  se il tipo non è deducibile o non è supportato → errore chiaro.

## Passi

1. Ambiente dei tipi locali (transpiler + compilatore), suite 28/28 invariata.
2. `to_json` + `__json_str` (interprete + transpiler + compilatore).
3. Esempio `native_json.logyx`, suite, MANUAL/manuale.html/sito/ROADMAP.
4. (Futuro) `from_json`: parser JSON → valori/record.
