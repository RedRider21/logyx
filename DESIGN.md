# Logyx — Documento di Design

- **Autore:** Daniele Deplano (RedRider21)
- **Licenza:** AGPL-3.0
- **Estensione file:** `.logyx` (breve `.lgx`)
- **Aggiornato:** 2026-09-28

Copia portabile e offline degli appunti di design. Il documento online (con diagramma) è a:
https://claude.ai/code/artifact/0aa46f19-cef5-4728-8e26-4b83867259cd

---

## Visione e obiettivo

Un solo linguaggio per fare ogni cosa: la versatilità e la concisione di Python, la leggerezza e la
comodità web di PHP, ma compilato a codice macchina e capace di girare sia sul server sia sul client.

L'obiettivo è cancellare la divisione tra "linguaggio server" e "linguaggio client". Oggi per il web
servono due linguaggi diversi; qui un solo sorgente, con una sola sintassi, gira nativo sul server e come
WebAssembly nel browser, con deploy semplice come PHP e velocità vicina al C.

## Versione 0 e piano di bootstrap

Logyx è alla **versione 0**. Python e Rust, che compaiono in questa fase, sono **impalcature
temporanee**, non parti del linguaggio finito:

- **Python** è il prototipo con cui "sentiamo" la semantica e iteriamo in fretta (lexer, parser,
  interprete, primo transpiler). È usa-e-getta.
- **Rust** è il trampolino di compilazione della v0: transpilare a Rust regala quasi gratis nativo,
  WASM, desktop e mobile prima di avere un backend nostro.

L'orizzonte è un Logyx **self-hosted**: il compilatore definitivo sarà scritto in Logyx stesso e
genererà direttamente codice macchina. A quel punto **né Python né Rust resteranno una dipendenza**.
Il percorso è quello classico del bootstrap: prototipo → compilatore che transpila → compilatore
self-hosted con backend proprio.

## Principi guida

Regole non negoziabili che decidono i dubbi di design futuri.

1. **Un sorgente, molti mondi.** Lo stesso codice gira nativo sul server e come WASM nel client, senza
   duplicazione e senza un secondo linguaggio.
2. **Facile per default, veloce a richiesta.** Dinamico e conciso come PHP/Python; i tipi si aggiungono solo
   dove serve prestazione (gradual typing).
3. **Sintassi esplicita, mai posizionale.** Delimitatori chiari, indentazione solo cosmetica, `;` opzionale.
4. **Batterie da web incluse.** Routing, template embedding, HTTP e database nella libreria standard, alla PHP.
5. **Interoperabilità come fondamento.** Accesso a C (lingua franca) e alle crates Rust dal primo giorno.
6. **Il linguaggio resta piccolo, l'ecosistema copre i mondi.** Il cuore sta in testa a una persona; la
   copertura arriva dalle librerie, non dal linguaggio che si gonfia.

Domanda aperta: gerarchia dei principi in caso di conflitto. Proposta iniziale: la sicurezza (nel confine
server/client) batte tutto, poi la semplicità (2, 3), poi la portabilità (1).

## Il nome

**Logyx**, scelto dopo verifica sul web (settembre 2026). Quasi tutti i nomi brevi ed evocativi sono già
usati da altri linguaggi; Logyx è risultato libero come nome di linguaggio.

- Onyx → **preso** (esiste già un linguaggio Onyx che compila a WASM: collisione diretta).
- Weave / Weft / Nexa / Basalt / Nyxa / Ryft / Kryon / Vyre → presi da altri linguaggi/progetti.
- λόγος / Logos → presi (Logo educativo; `logos`, generatore di lexer in Rust — proprio il nostro dominio).
- **Logyx** → libero. Conserva la radice λόγος (parola, ragione, linguaggio) e il suono `-yx` di Onyx.
- Obsyd → libero (era la seconda scelta; deriva da obsidian).

## Sintassi

Non posizionale: la struttura la danno i delimitatori, non l'indentazione. I rientri sono solo per
leggibilità e il `;` è opzionale (inserito automaticamente dal compilatore).

Le porzioni che devono girare nel browser si marcano con blocchi espliciti `@start-client ... @end-client`:
fuori da essi è tutto server, così si vede a colpo d'occhio dove passa il confine. Un decoratore
`@server` / `@client` è la variante breve per un'intera funzione.

```
route "/todo":                     # server (come PHP)
    lista = db.query("SELECT * FROM todo")

    render <html>
      <ul>{ for t in lista: <li>{t.testo}</li> }</ul>

      @start-client
        button.onClick = () => { conta += 1 }   # diventa WASM
      @end-client
    </html>

# con tipi, dove serve prestazione
def somma(a: int, b: int) -> int:
    return a + b
```

## Sistema di tipi ed esecuzione

Gradual typing: dinamico e conciso per default; le annotazioni di tipo sono opzionali e servono a chi vuole
controllo e prestazione. Il compilatore usa la type inference dove i tipi non sono scritti.

Due modalità di esecuzione:

- **Interprete** per lo sviluppo rapido (sensazione Python, il linguaggio gira subito).
- **Compilazione a codice macchina** per la produzione, transpilando verso Rust e lasciando che `rustc`
  faccia il lavoro pesante (ottimizzazione, sicurezza di memoria, linking, generazione WASM).

La scelta di transpilare a Rust regala quasi gratis server nativo, client WASM e i target desktop/mobile,
perché sono tutti bersagli che la toolchain Rust già raggiunge.

## Gestione degli errori

Deciso (2026-09-28): **errori come valori** (modello Result), non eccezioni. È la scelta coerente con la
compilazione a Rust — dove non esistono le eccezioni — e con il principio "i pericoli sono espliciti".

- Una funzione fallibile ha tipo `-> T | error` (dedotto se il corpo usa `fail`/`?`).
- `fail "msg"` restituisce un errore; `espr?` lo propaga; `match e { ok v {…} err e {…} }` lo gestisce.
- Traduzione a Rust uno-a-uno: `T | error` → `Result<T, String>`, `fail` → `Err`, `?` → `?`,
  `match ok/err` → `match Ok/Err`. La concisione delle eccezioni si recupera con `?`, senza perdere
  l'esplicitezza. Distinzione sana: errori previsti → Result; bug irrecuperabili → arresto.

## Confine server/client

Server e client sono due macchine diverse che non condividono memoria e si parlano solo via rete. Il
linguaggio non finge che il confine non esista: lo rende comodo da attraversare senza nasconderne i pericoli.

Tre regole inviolabili:

1. **Solo dati serializzabili attraversano.** Numeri, stringhe, liste, oggetti semplici sì; connessioni al
   DB, handle di file, funzioni no. Il compilatore verifica e rifiuta ciò che non può viaggiare.
2. **Il client non è mai fidato.** Nessun segreto nel codice client; ogni dato che arriva dal browser va
   validato di nuovo sul server. Il confine è anche un confine di sicurezza.
3. **Attraversare costa.** Ogni salto è un viaggio di rete: il design lo rende visibile (i blocchi
   `@start-client`).

**Modello di default: server-driven** (stile LiveView). Lo stato e la logica vivono sul server, il client
resta quasi vuoto, il server rimanda solo le differenze del DOM. Massima leggerezza stile PHP e sicurezza
quasi gratis. Per interazioni istantanee, seconda marcia con modello RPC/idratazione.

| Modello | Come funziona | Quando |
| --- | --- | --- |
| Server-driven (default) | Stato sul server, il client riceve i diff del DOM | Gran parte delle app |
| RPC trasparente | Il client chiama funzioni `@server` come locali; sotto c'è la rete | App reattive, offline |
| Idratazione (SSR) | Il server genera l'HTML, il WASM lo anima | Siti di contenuto, SEO |

## Multi-target

Un solo nucleo, molti bersagli di compilazione. Poiché si transpila a Rust, si ereditano tutti i target che
Rust già raggiunge.

| Bersaglio | Cosa ci fai | Come ci arrivi |
| --- | --- | --- |
| Server nativo (Linux x86/ARM) | Backend web, servizi, API | Rust → nativo |
| Browser (WASM) | Frontend web | Rust → WASM |
| Desktop (Win/Mac/Linux) | App con finestre | Rust → nativo |
| Mobile (Android/iOS) | App con touch e sensori | Rust → ARM |

Distinzione da incidere nella pietra:

- **Logica portabile** — calcoli, algoritmi, tipi, regole di business: gira identica ovunque. È il grosso
  di un programma.
- **Capacità di piattaforma** — DOM nel browser, finestre e filesystem sul desktop, GPS e fotocamera sul
  mobile: diverse per forza, esposte da moduli specifici.

Sul web il "client" è il browser, dove non si può installare nulla: da qui l'obbligo di WASM. Su app
desktop/mobile il runtime nativo può stare su entrambi i lati.

## Ecosistema e interoperabilità

Aggancio a due ecosistemi nativi, senza tradire la compilazione a macchina:

- **FFI con C — base universale.** Il C ha una ABI stabile: la lingua franca con cui tutti i linguaggi si
  parlano. Basta dichiarare la firma e il linker fa il resto.
- **Crates Rust — cavallo di battaglia.** Rust non ha una ABI stabile, quindi le crates si usano solo
  dall'interno del mondo Rust: è il motivo tecnico per cui il backend transpila a Rust invece di puntare a
  LLVM.

```
extern "C" {
    fn sqrt(x: float) -> float
}
```

Nota: "compilato nativo puro" e "importo qualsiasi libreria Python al volo" sono in conflitto tecnico. Qui
vince il nativo; l'ecosistema Python resta un'opzione futura via ponte, non un fondamento.

## Architettura del compilatore

Non un compilatore monolitico, ma un frontend unico con più backend. Consiglio pratico per la **v0**: **prototipo in
Python** (lexer, parser, interprete: programmi veri in pochi giorni), poi un backend che **transpila a
Rust** per raggiungere subito nativo e WASM. È un trampolino: l'obiettivo è arrivare a un **compilatore
self-hosted**, scritto in Logyx, con backend proprio e senza dipendere da Python né da Rust.

Pipeline:

```
sorgente .logyx
      │
  Lexer → Parser → AST → Type checker
      │
  Backend: genera codice Rust
      │
    rustc
      ├── Nativo (server · desktop · mobile)
      └── WASM (browser)
```

Il frontend è nostro; da `rustc` in poi il lavoro pesante è già fatto, e i bersagli nativo e WASM arrivano
dalla stessa toolchain.

## Decisioni prese

| Tema | Decisione |
| --- | --- |
| Nome | Logyx (estensione `.logyx`) |
| Ecosistema | FFI con C + interop con le crates Rust |
| Backend | v0: transpiling verso Rust (poi `rustc`). Orizzonte: backend proprio self-hosted (niente Rust) |
| Host del compilatore | v0: prototipo in Python. Poi compilatore self-hosted in Logyx, senza dipendere da Python né Rust |
| Sintassi | Non posizionale, delimitatori espliciti, `;` opzionale |
| Isole client | Blocchi `@start-client / @end-client`; default server |
| Confine server/client | Server-driven di default; RPC/idratazione come seconda marcia |
| Target | Server nativo, browser WASM, desktop, mobile |
| Tipi | Gradual typing |
| Errori | Valori (Result): `fail` / `?` / `match ok/err`; transpila a `Result<T, String>` |
| Autore | Daniele Deplano (RedRider21) — header su ogni sorgente |
| Licenza | AGPL-3.0 |

## Domande ancora aperte

- Gerarchia dei principi quando confliggono.
- Grammatica precisa: parole chiave, forma dei blocchi (`{}` o `end`), regole del `;` automatico.
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il modello server-driven).

## Prossimi passi

1. Definire la grammatica di un programma minimo: parole chiave, forma dei blocchi, regole del `;` automatico.
2. Scrivere il primo "ciao mondo" multi-target: stesso sorgente che risponde a una route sul server e mostra
   qualcosa di interattivo nel client.
3. Prototipo del frontend (lexer + parser + interprete) in Python o TypeScript, per "sentire" il linguaggio
   prima di scrivere il compilatore Rust.
