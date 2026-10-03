# Mini-design — Record (tipi con campi)

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Data:** 2026-10-03

Obiettivo: tipi con campi nominati (come gli `struct`), per modellare i dati. È il prerequisito pratico
verso JSON e verso programmi "reali".

## Sintassi

```
record Persona {
    nome: string,
    eta: int
}

fn descrivi(p: Persona) -> string {
    return p.nome + " ha " + str(p.eta) + " anni"
}

fn main() {
    ada = Persona("Ada", 36)          // costruzione (posizionale)
    print(descrivi(ada))
    print("nome: {ada.nome}, eta: {ada.eta}")   // accesso ai campi
}
```

- **Definizione:** parola chiave nuova `record Nome { campo: tipo, ... }`. I campi hanno **tipi
  espliciti** (servono a generare la `struct` Rust). Campi ammessi nella v0: scalari e `string` (record
  annidati: fase successiva).
- **Costruzione:** `Nome(v1, v2)` — **posizionale**, nell'ordine dei campi. È sintatticamente una chiamata:
  si sceglie la forma con parentesi per **evitare l'ambiguità** con i blocchi `{ }` (che affliggerebbe
  `Nome { ... }` dentro un `if`/`for`). Il backend la riconosce come costruzione perché `Nome` è un record.
- **Accesso ai campi:** `p.campo` — nuovo operatore postfisso `.` (nuovo token `DOT`; oggi il `.` compare
  solo dentro i numeri float).

## Rappresentazione in Rust

Per ogni record si emette, **prima delle funzioni**:

```
#[derive(Clone)]
struct Persona { nome: String, eta: i64 }
```

- Costruzione `Persona("Ada", 36)` → `Persona { nome: "Ada".to_string(), eta: 36i64 }`.
- Accesso `p.nome` → `p.nome.clone()` (coerente con l'opzione "clone uniforme" già adottata).
- `ty("Persona")` → `"Persona"`: il controllo dei tipi deve accettare i nomi di record oltre ai tipi base.

## Il nodo critico: passare i record alle funzioni (il *move*)

`String` e i record **non sono `Copy`**: passare una variabile a una funzione la **muove**, e un uso
successivo non compila. Esempio: `descrivi(ada)` poi `ada.nome` → errore.

**Soluzione (clone uniforme, estesa alle chiamate):** quando l'argomento di una chiamata è una **variabile
nuda** (un identificatore), si genera `arg.clone()`. Per i tipi `Copy` (`i64`/`f64`/`bool`) `clone()` è
gratuito dopo l'ottimizzazione; per `String`/record copia il necessario. Gli argomenti che sono già valori
propri (letterali, `p.campo`, `xs[i]`, risultati di espressioni) restano invariati.

- Conformità preservata: `fib(i)` → `fib(i.clone())` dà lo stesso valore; la suite confronta l'output.
- Da verificare col primo commit del cambiamento sulle chiamate (i 27 esempi devono restare 27/27).

## Interprete

- Un record è un valore con un'etichetta di tipo e i campi, es. un dict `{"__record__": "Persona",
  "nome": "Ada", "eta": 36}` (o una piccola classe `RecordValue`).
- `RecordDef` registrato in fase di `load` (nome → lista ordinata dei campi).
- Costruzione: `ex_Call` su un identificatore che è un record → crea il valore mappando gli argomenti
  posizionalmente ai campi.
- Accesso: nuovo nodo `Field(target, name)` → `ex_Field` restituisce `valore[name]`.
- `logyx_str` di un record: `Persona(nome: "Ada", eta: 36)` (forma leggibile).

## Tipi e inferenza

- I **parametri** di tipo record vanno **annotati** (`p: Persona`): l'uso `p.nome` da solo non basta a
  inferire di quale record si tratti.
- `p.campo` ha il tipo del campo, noto dalla tabella dei record (nome → {campo: tipo}); serve al backend
  per l'inferenza del tipo di ritorno e per le concatenazioni.
- La costruzione `Persona(...)` ha tipo `Persona`.

## Scope della v0

- Campi scalari e `string`. **No** record annidati, **no** liste/mappe di record (fase successiva:
  è un altro giro di "clone uniforme" sui contenitori con elemento record).
- Record usabili come variabili locali, **parametri e valori di ritorno** (grazie al clone sugli argomenti).

## Passi

1. **Cambiamento sulle chiamate**: argomenti identificatore → `.clone()` (transpiler + compilatore).
   Commit a sé, suite 27/27 verde (cambia solo il `.rs`).
2. Lexer/parser/AST: token `DOT`, keyword `record`, nodi `RecordDef` e `Field`, parsing della definizione
   e dell'accesso ai campi.
3. Interprete: `RecordDef`, costruzione, `ex_Field`, `logyx_str`.
4. Backend: emissione delle `struct`, costruzione, accesso ai campi, `ty()` sui nomi di record, tabella
   dei record per l'inferenza.
5. Esempio `native_record.logyx`, suite, `MANUAL.md` + `manuale.html` + sito + `ROADMAP.md`.
