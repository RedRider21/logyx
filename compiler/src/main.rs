// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! logyxc — compilatore di Logyx (Fase 1 del bootstrap).
//!
//! Comandi:
//!   logyxc tokens <file.logyx>    stampa i token del file
//!   logyxc parse  <file.logyx>    stampa l'AST del file
//!   logyxc gen    <file.logyx>    stampa il codice Rust generato
//!   logyxc build  <file.logyx>    genera il .rs e compila con rustc (se presente)

mod ast;
mod codegen;
mod error;
mod lexer;
mod modules;
mod parser;
mod token;

use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
    let result = match (cmd, args.get(2)) {
        ("tokens", Some(p)) => run_tokens(p),
        ("parse", Some(p)) => run_parse(p),
        ("gen", Some(p)) => run_gen(p),
        ("build", Some(p)) => run_build(p),
        _ => {
            eprintln!("uso: logyxc <tokens|parse|gen|build> <file.logyx>");
            exit(2);
        }
    };
    if let Err(e) = result {
        eprintln!("Errore: {e}");
        exit(1);
    }
}

fn generate_rust(path: &str) -> Result<String, error::LogyxError> {
    let items = modules::load_program(path)?;
    codegen::Codegen::new().generate(&items)
}

fn run_gen(path: &str) -> Result<(), error::LogyxError> {
    print!("{}", generate_rust(path)?);
    Ok(())
}

fn run_build(path: &str) -> Result<(), error::LogyxError> {
    let rust = generate_rust(path)?;
    let out_rs = change_ext(path, "rs");
    std::fs::write(&out_rs, &rust)
        .map_err(|e| error::LogyxError::new(format!("{out_rs}: {e}")))?;
    println!("// Rust generato in {out_rs}");
    if which_rustc() {
        let bin = strip_ext(path) + "_bin";
        let comp = std::process::Command::new("rustc")
            .args(["-O", &out_rs, "-o", &bin])
            .output()
            .map_err(|e| error::LogyxError::new(format!("rustc: {e}")))?;
        if !comp.status.success() {
            return Err(error::LogyxError::new(format!(
                "rustc ha segnalato errori:\n{}",
                String::from_utf8_lossy(&comp.stderr)
            )));
        }
        println!("──── esecuzione del binario nativo ────");
        let run = std::process::Command::new(&bin)
            .output()
            .map_err(|e| error::LogyxError::new(format!("{bin}: {e}")))?;
        print!("{}", String::from_utf8_lossy(&run.stdout));
    } else {
        println!("──── rustc non installato ────");
        println!("Per compilare:  rustc -O {out_rs} -o app && ./app");
    }
    Ok(())
}

fn which_rustc() -> bool {
    std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn strip_ext(path: &str) -> String {
    match path.rfind('.') {
        Some(i) => path[..i].to_string(),
        None => path.to_string(),
    }
}

fn change_ext(path: &str, ext: &str) -> String {
    format!("{}.{}", strip_ext(path), ext)
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
