# Logyx — Grammatica v0

- **Autore:** Daniele Deplano (RedRider21) · **Licenza:** AGPL-3.0 · **Aggiornato:** 2026-09-28

Prima bozza della grammatica. Volutamente piccola: copre il minimo per scrivere il "ciao mondo"
multi-target e crescere per gradi. Le scelte qui sono coerenti con i principi in `DESIGN.md`.

## Scelte di base (v0)

| Aspetto | Decisione | Motivo |
| --- | --- | --- |
| Blocchi | Graffe `{ }` | Sintassi non posizionale, nessuna ambiguità tab/spazi |
| Fine istruzione | Fine riga oppure `;` (opzionale) | Leggerezza; il `;` non è obbligatorio |
| Commenti | `//` a riga singola, `/* ... */` a blocco | Familiari, coerenti con le graffe |
| Indentazione | Solo cosmetica | Non cambia il significato |
| Booleani / nulla | `true`, `false`, `nil` | — |
| Logici | `and`, `or`, `not` | Leggibili, stile Python |
| Stringhe | `"..."` con interpolazione `"Ciao {nome}"` | Comodo per il web |

## Elementi lessicali

```
Identificatore := lettera (lettera | cifra | "_")*
Intero         := cifra+
Float          := cifra+ "." cifra+
Stringa        := '"' ( carattere | "{" espressione "}" )* '"'
Booleano       := "true" | "false"
Nullo          := "nil"
```

Parole chiave riservate (v0): `fn return if else while for in const import fail match break continue record enum use extern and or not true false nil route render`
più i marcatori di confine `@server @client @start-client @end-client`. Dentro `match` i rami sono
etichettati da `ok` ed `err` (parole contestuali), e `error` compare nei tipi fallibili `T | error`.

## Dichiarazioni e tipi

```
// variabile: dichiara e assegna; il tipo è opzionale (gradual typing)
x = 10
nome: string = "Ada"
const PI = 3.14159

// tipi base:  int  float  bool  string  nil
// collezioni: [T] lista        {K: V} mappa
numeri: [int] = [1, 2, 3]
utente: {string: string} = {"nome": "Ada"}
```

## Funzioni

```
fn saluta(nome: string) -> string {
    return "Ciao " + nome
}

// i tipi sono opzionali: questa è valida ed è dedotta dal compilatore
fn doppio(n) {
    return n * 2
}

// punto d'ingresso di un programma eseguibile
fn main() {
    print(saluta("mondo"))
}
```

## Controllo di flusso

```
if x > 0 {
    print("positivo")
} else if x == 0 {
    print("zero")
} else {
    print("negativo")
}

while x > 0 {
    x = x - 1
}

for n in numeri {
    print(n)
}
```

## Web: route, template e isole client

`route` definisce un indirizzo a cui risponde il **server**. `render` produce HTML con interpolazione
`{espressione}` e blocchi di controllo `{ for x in xs { <li>{x}</li> } }`.

Il codice dentro `@start-client ... @end-client` viene compilato per il **client** (WASM nel browser).
Fuori da quei blocchi è tutto server. API client v0 (provvisoria): `on "<evento>" of "<selettore>" { ... }`
e `set text of "<selettore>" to <espressione>`.

```
route "/" {
    saluto = "Ciao, mondo!"          // SERVER (nativo)

    render <html>
      <body>
        <h1>{saluto}</h1>
        <button id="btn">Cliccami</button>
        <p id="out">—</p>

        @start-client                 // CLIENT (WASM)
          clic = 0
          on "click" of "#btn" {
              clic = clic + 1
              set text of "#out" to "Premuto {clic} volte"
          }
        @end-client
      </body>
    </html>
}
```

## Grammatica sintetica (EBNF, v0)

```
programma    := elemento*
elemento     := funzione | route | dichiarazione | import | record | enum | useRust | externRust
import       := "import" STRINGA           // percorso di un file .logyx
record       := "record" IDENT "{" campo ("," campo)* "}"
campo        := IDENT ":" tipo
enum         := "enum" IDENT "{" IDENT ("," IDENT)* "}"   // varianti senza payload (v0)
useRust      := "use" "rust" STRINGA "=" STRINGA               // dipendenza crate
externRust   := "extern" "rust" "fn" IDENT "(" parametri ")" "->" tipo "=" STRINGA  // corpo Rust
// costruzione: Nome(v1, v2, ...) posizionale · accesso campo: espressione "." IDENT

funzione     := "fn" IDENT "(" parametri? ")" ("->" tipo)? blocco
parametri    := parametro ("," parametro)*
parametro    := IDENT (":" tipo)?

route        := "route" STRINGA ("(" IDENT ")")? blocco

dichiarazione:= ("const")? IDENT (":" tipo)? "=" espressione
assegnazione := bersaglio ("=" | "+=" | "-=" | "*=" | "/=" | "%=") espressione
bersaglio    := IDENT | IDENT "[" espressione "]"

blocco       := "{" istruzione* "}"
istruzione   := dichiarazione | assegnazione | if | while | for
              | "return" espressione? | "fail" espressione | match
              | "break" | "continue"
              | espressione | render | isolaClient
isolaClient  := "@start-client" istruzione* "@end-client"

match        := matchValore | matchErrori
matchValore  := "match" espressione "{" (espressione blocco)* ("else" blocco)? "}"
matchErrori  := "match" espressione "{" "ok" IDENT blocco "err" IDENT blocco "}"

if           := "if" espressione blocco ("else" "if" espressione blocco)* ("else" blocco)?
while        := "while" espressione blocco
for          := "for" IDENT "in" espressione blocco

render       := "render" template
espressione  := ... (letterali, chiamate, operatori aritmetici/confronto/logici, postfisso "?")
                // e? propaga l'errore: se e è un errore, esce dalla funzione; altrimenti dà il valore
tipo         := tipoBase ("|" "error")?          // "T | error" = fallibile (Result in Rust)
tipoBase     := "int" | "float" | "bool" | "string" | "nil"
              | "[" tipoBase "]" | "{" tipoBase ":" tipoBase "}"
```

Le parti marcate `...` (precedenze degli operatori, template dettagliato) saranno formalizzate quando
scriveremo il parser del prototipo.
