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

| File | Contenuto |
| --- | --- |
| `README.md` | Questo file: cos'è Logyx e come riprendere |
| `DESIGN.md` | Tutti gli appunti di design in forma completa e portabile |
| `CONTEXT.md` | Punto di ripresa: dove siamo e qual è il prossimo passo |
| `LICENSE` | Testo completo della licenza AGPL-3.0 |

## Come continuare il lavoro (anche su un altro PC)

1. Copia questa cartella `logyx/` dove vuoi (chiavetta, altro computer, oppure `git push` su GitHub).
2. Sull'altro PC, apri Claude Code **dentro questa cartella**.
3. Fai leggere `CONTEXT.md` e `DESIGN.md`: contengono tutto il contesto per proseguire senza perdere nulla.

La cartella è autosufficiente e versionata con git: non dipende da servizi esterni.
Il documento online è una comodità in più per consultare e commentare; la copia portabile è questa.
