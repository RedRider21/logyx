// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! logyxc — compilatore di Logyx (Fase 1 del bootstrap).
//!
//! Per ora espone lexer e parser a scopo di sviluppo e verifica:
//!   logyxc tokens <file.logyx>    stampa i token del file
//!   logyxc parse  <file.logyx>    stampa l'AST del file

mod ast;
mod error;
mod lexer;
mod parser;
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
    if args.len() >= 3 && args[1] == "parse" {
        if let Err(e) = run_parse(&args[2]) {
            eprintln!("Errore: {e}");
            exit(1);
        }
        return;
    }
    eprintln!("uso: logyxc <tokens|parse> <file.logyx>");
    exit(2);
}

fn run_parse(path: &str) -> Result<(), error::LogyxError> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| error::LogyxError::new(format!("{path}: {e}")))?;
    let toks = lexer::Lexer::new(&src, path).tokenize()?;
    let items = parser::Parser::new(toks, path.to_string()).parse()?;
    for it in &items {
        println!("{it:#?}");
    }
    Ok(())
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
