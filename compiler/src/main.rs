// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! logyxc — compilatore di Logyx (Fase 1 del bootstrap).
//!
//! Per ora espone il lexer a scopo di sviluppo e verifica:
//!   logyxc tokens <file.logyx>    stampa i token del file

mod error;
mod lexer;
mod token;

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "tokens" {
        if let Err(e) = run_tokens(&args[2]) {
            eprintln!("Errore: {e}");
            exit(1);
        }
        return;
    }
    eprintln!("uso: logyxc tokens <file.logyx>");
    exit(2);
}

fn run_tokens(path: &str) -> Result<(), error::LogyxError> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| error::LogyxError::new(format!("{path}: {e}")))?;
    let toks = lexer::Lexer::new(&src, path).tokenize()?;
    for t in &toks {
        println!("{:>4}:{:<3} {:?}", t.line, t.col, t.kind);
    }
    println!("({} token)", toks.len());
    Ok(())
}
