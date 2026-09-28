# Logyx

Un nuovo linguaggio di programmazione: versatile e conciso come Python, comodo per il web come PHP,
ma **compilato a codice macchina** e capace di girare **sia sul server sia sul client**.
Un solo sorgente, una sola sintassi, molti bersagli (server nativo, browser via WASM, desktop, mobile).

Il nome unisce la radice greca **λόγος** (parola, ragione, linguaggio) al suono `-yx` (dalla pietra onyx).

- **Estensione dei file:** `.logyx` (breve: `.lgx`)
- **Autore:** Daniele Deplano (RedRider21)
- **Licenza:** AGPL-3.0 (vedi `LICENSE`)
- **Stato:** fase di design (nessun codice del compilatore ancora scritto)
- **Documento di design online:** https://claude.ai/code/artifact/0aa46f19-cef5-4728-8e26-4b83867259cd

## Intestazione di copyright per i file di programma

Ogni file sorgente del progetto deve iniziare con:

```
Copyright (C) 2026 Daniele Deplano (RedRider21)
SPDX-License-Identifier: AGPL-3.0-or-later
```

## Com'è organizzata questa cartella

| File / cartella | Contenuto |
| --- | --- |
| `README.md` | Questo file: cos'è Logyx e come riprendere |
| `MANUAL.md` | Manuale del linguaggio e delle funzioni (aggiornato) |
| `GRAMMAR.md` | Grammatica v0 (EBNF) |
| `DESIGN.md` | Appunti di design completi e portabili |
| `CONTEXT.md` | Punto di ripresa: dove siamo e prossimo passo |
| `prototype/` | Interprete/transpiler in Python (lexer, parser, interprete, rustgen) |
| `examples/` | Programmi di esempio `.logyx` |
| `docs/` | Documenti esportati (es. il documento di design in `.md`/`.pdf`) |
| `LICENSE` | Testo completo della licenza AGPL-3.0 |

## Prerequisiti

Il progetto è tutto qui dentro; sulla macchina servono solo gli strumenti standard:

- **Python 3.8+** — per eseguire il prototipo (nessuna dipendenza esterna).
- **Rust** (`rustc`) — *opzionale*, solo per compilare a nativo con `build`. Senza Rust, `build` genera
  comunque il codice `.rs` da compilare altrove.

## Come continuare il lavoro (anche su un altro PC)

1. Copia questa cartella `logyx/` dove vuoi (chiavetta, altro computer, oppure `git push` su GitHub).
2. Sull'altro PC, apri Claude Code **dentro questa cartella**.
3. Fai leggere `CONTEXT.md` e `DESIGN.md`: contengono tutto il contesto per proseguire senza perdere nulla.

La cartella è autosufficiente e versionata con git: non dipende da servizi esterni.
Il documento online è una comodità in più per consultare e commentare; la copia portabile è questa.
