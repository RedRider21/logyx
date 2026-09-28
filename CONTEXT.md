# Punto di ripresa — Logyx

Aggiornato: 2026-09-28

## Come riprendere in una nuova sessione (anche su un altro PC)

1. Apri Claude Code dentro la cartella `logyx/`.
2. Leggi questo file e poi `DESIGN.md`.
3. Riprendi dal "Prossimo passo" qui sotto.

## Dove siamo

Fase di **design**. Nessun codice del compilatore ancora scritto. Il nome, la licenza, l'autore e le scelte
architetturali di fondo sono fissati (vedi la tabella "Decisioni prese" in `DESIGN.md`).

## Deciso e bloccato

- **Nome:** Logyx — estensione `.logyx` (breve `.lgx`).
- **Autore:** Daniele Deplano (RedRider21). Nessuna firma di terzi nei sorgenti o nei commit.
- **Licenza:** AGPL-3.0. Header su ogni file di programma:
  `Copyright (C) 2026 Daniele Deplano (RedRider21)` + `SPDX-License-Identifier: AGPL-3.0-or-later`.
- **Backend:** transpiling verso Rust, poi `rustc` (nativo + WASM). Non LLVM diretto all'inizio.
- **Host del compilatore:** prototipo in Python/TypeScript, compilatore vero in Rust.
- **Confine server/client:** modello server-driven di default.
- **Ecosistema:** FFI con C + interop crates Rust.

## Prossimo passo

Definire la **grammatica** di un programma minimo (parole chiave, forma dei blocchi, regole del `;`
automatico) e scrivere il primo **"ciao mondo" multi-target**.

## Questioni aperte da decidere

- Gerarchia dei principi in caso di conflitto.
- Forma dei blocchi: graffe `{}` oppure `end`.
- Formato di serializzazione e trasporto oltre il confine (WebSocket per il server-driven).
