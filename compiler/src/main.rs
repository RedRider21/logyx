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
        ("build-wasm", Some(p)) => run_build_wasm(p),
        _ => {
            eprintln!("uso: logyxc <tokens|parse|gen|build|build-wasm> <file.logyx>");
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
    let items = modules::load_program(path)?;
    let mut cg = codegen::Codegen::new();
    let rust = cg.generate(&items)?;
    let deps = cg.deps.clone();
    if deps.is_empty() {
        build_rustc(path, &rust)
    } else {
        build_cargo(path, &rust, &deps)
    }
}

/// Fase 0 web→WASM: compila le funzioni del nucleo a WebAssembly (tipi numerici)
/// e genera una pagina HTML che le invoca dal browser.
fn run_build_wasm(path: &str) -> Result<(), error::LogyxError> {
    use ast::Item;
    let items = modules::load_program(path)?;
    let mut cg = codegen::Codegen::new();
    let rust = cg.generate(&items)?;
    let deps = cg.deps.clone();
    let names: Vec<String> = items
        .iter()
        .filter_map(|it| if let Item::Func(f) = it { Some(f.name.clone()) } else { None })
        .collect();
    let exports = cg.numeric_exports(&names);
    if exports.is_empty() {
        return Err(error::LogyxError::new(
            "build-wasm: nessuna funzione con firma numerica da esportare \
             (la Fase 0 supporta int/float/bool)",
        ));
    }
    // wrapper esportati verso WASM. int -> i32 (così JS usa Number, non BigInt).
    let wasm_ty = |t: &str| if t == "float" { "f64" } else { "i32" };
    let arg_in = |n: &str, t: &str| match t {
        "int" => format!("{n} as i64"),
        "bool" => format!("({n} != 0)"),
        _ => n.to_string(), // float
    };
    let mut wrappers = String::new();
    for e in &exports {
        let params = e
            .params
            .iter()
            .map(|(n, t)| format!("{}: {}", n, wasm_ty(t)))
            .collect::<Vec<_>>()
            .join(", ");
        let argvals = e
            .params
            .iter()
            .map(|(n, t)| arg_in(n, t))
            .collect::<Vec<_>>()
            .join(", ");
        let call = format!("{}({})", e.name, argvals);
        let body = match e.ret.as_str() {
            "int" => format!("({call}) as i32"),
            "bool" => format!("({call}) as i32"),
            _ => call, // float
        };
        wrappers += &format!(
            "#[export_name = \"{}\"]\npub extern \"C\" fn __wasm_{}({}) -> {} {{ {} }}\n\n",
            e.name,
            e.name,
            params,
            wasm_ty(&e.ret),
            body
        );
    }
    let lib = format!("#![allow(dead_code, unused)]\n\n{}\n{}", rust, wrappers);
    // crate cdylib
    let dir = format!("{}_wasm", strip_ext(path));
    std::fs::create_dir_all(format!("{dir}/src")).map_err(|e| error::LogyxError::new(format!("{dir}: {e}")))?;
    let mut toml = String::from(
        "[package]\nname = \"logyxwasm\"\nversion = \"0.0.1\"\nedition = \"2021\"\n\n\
         [lib]\ncrate-type = [\"cdylib\"]\npath = \"src/lib.rs\"\n\n[dependencies]\n",
    );
    let mut keys: Vec<&String> = deps.keys().collect();
    keys.sort();
    for k in &keys {
        toml += &format!("{} = {}\n", k, deps[*k]);
    }
    toml += "\n[profile.release]\nopt-level = \"z\"\nlto = true\n";
    std::fs::write(format!("{dir}/Cargo.toml"), toml)
        .map_err(|e| error::LogyxError::new(format!("{dir}/Cargo.toml: {e}")))?;
    std::fs::write(format!("{dir}/src/lib.rs"), lib)
        .map_err(|e| error::LogyxError::new(format!("{dir}/src/lib.rs: {e}")))?;
    if !which("cargo") {
        return Err(error::LogyxError::new("cargo non installato"));
    }
    let comp = std::process::Command::new("cargo")
        .args(["build", "--release", "--target", "wasm32-unknown-unknown"])
        .current_dir(&dir)
        .output()
        .map_err(|e| error::LogyxError::new(format!("cargo: {e}")))?;
    if !comp.status.success() {
        return Err(error::LogyxError::new(format!(
            "cargo ha segnalato errori:\n{}",
            String::from_utf8_lossy(&comp.stderr)
        )));
    }
    let wasm_src = format!("{dir}/target/wasm32-unknown-unknown/release/logyxwasm.wasm");
    let base = strip_ext(path);
    let wasm_out = format!("{base}.wasm");
    let html_out = format!("{base}.html");
    std::fs::copy(&wasm_src, &wasm_out).map_err(|e| error::LogyxError::new(format!("{wasm_out}: {e}")))?;
    let wasm_name = std::path::Path::new(&wasm_out)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or(wasm_out.clone());
    std::fs::write(&html_out, wasm_demo_html(&wasm_name))
        .map_err(|e| error::LogyxError::new(format!("{html_out}: {e}")))?;
    let fn_list: Vec<String> = exports
        .iter()
        .map(|e| format!("{}({})", e.name, e.params.len()))
        .collect();
    println!("// WASM generato: {wasm_out}");
    println!("// Pagina demo: {html_out}  (funzioni: {})", fn_list.join(", "));
    let html_name = wasm_name.replace(".wasm", ".html");
    println!("Apri nel browser servendo la cartella, es.:");
    println!("  python3 -m http.server --directory {} 8000", parent_dir(&html_out));
    println!("  poi apri http://localhost:8000/{html_name}");
    Ok(())
}

fn parent_dir(p: &str) -> String {
    std::path::Path::new(p)
        .parent()
        .map(|d| d.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| ".".to_string())
}

fn wasm_demo_html(wasm_name: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="it">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Logyx → WebAssembly</title>
<style>
  body{{font-family:-apple-system,Segoe UI,Roboto,sans-serif; max-width:640px; margin:40px auto; padding:0 20px; color:#171a2b}}
  h1{{letter-spacing:-.02em}}
  .fn{{border:1px solid #e3e5ef; border-radius:12px; padding:14px 16px; margin:12px 0}}
  code{{font-family:ui-monospace,monospace; color:#5b4be1; font-weight:600}}
  input{{width:70px; padding:4px 6px; margin:0 2px}}
  button{{margin-left:8px; padding:5px 12px; border-radius:8px; border:1px solid #5b4be1; background:#5b4be1; color:#fff; cursor:pointer}}
  .res{{font-weight:700; color:#5b4be1; margin-left:8px}}
</style>
</head>
<body>
<h1>Logyx → WebAssembly</h1>
<p>Funzioni del nucleo di Logyx compilate a WASM e invocate direttamente nel browser.</p>
<div id="app">caricamento…</div>
<script>
(async () => {{
  const app = document.getElementById("app");
  try {{
    const bytes = await (await fetch("{wasm}")).arrayBuffer();
    const {{ instance }} = await WebAssembly.instantiate(bytes, {{}});
    const ex = instance.exports;
    app.innerHTML = "";
    for (const name of Object.keys(ex)) {{
      const f = ex[name];
      if (typeof f !== "function") continue;
      const arity = f.length;
      const div = document.createElement("div"); div.className = "fn";
      const lab = document.createElement("span"); lab.innerHTML = "<code>" + name + "</code> ( ";
      div.appendChild(lab);
      const inputs = [];
      for (let i = 0; i < arity; i++) {{
        const inp = document.createElement("input"); inp.type = "number"; inp.value = "0";
        inputs.push(inp); div.appendChild(inp);
        if (i < arity - 1) div.appendChild(document.createTextNode(", "));
      }}
      div.appendChild(document.createTextNode(" )"));
      const btn = document.createElement("button"); btn.textContent = "calcola";
      const res = document.createElement("span"); res.className = "res";
      btn.onclick = () => {{ res.textContent = "= " + f(...inputs.map(x => Number(x.value))); }};
      div.appendChild(btn); div.appendChild(res);
      app.appendChild(div);
    }}
  }} catch (e) {{
    app.textContent = "Errore nel caricare il WASM: " + e + " (servi la pagina via http, non file://)";
  }}
}})();
</script>
</body>
</html>
"#,
        wasm = wasm_name
    )
}

fn build_rustc(path: &str, rust: &str) -> Result<(), error::LogyxError> {
    let out_rs = change_ext(path, "rs");
    std::fs::write(&out_rs, rust).map_err(|e| error::LogyxError::new(format!("{out_rs}: {e}")))?;
    println!("// Rust generato in {out_rs}");
    if !which("rustc") {
        println!("──── rustc non installato ────");
        println!("Per compilare:  rustc -O {out_rs} -o app && ./app");
        return Ok(());
    }
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
    Ok(())
}

fn build_cargo(
    path: &str,
    rust: &str,
    deps: &std::collections::HashMap<String, String>,
) -> Result<(), error::LogyxError> {
    let dir = format!("{}_cargo", strip_ext(path));
    std::fs::create_dir_all(format!("{dir}/src"))
        .map_err(|e| error::LogyxError::new(format!("{dir}: {e}")))?;
    let mut keys: Vec<&String> = deps.keys().collect();
    keys.sort();
    let mut toml = String::from(
        "[package]\nname = \"logyxprog\"\nversion = \"0.0.1\"\nedition = \"2021\"\n\n[dependencies]\n",
    );
    for k in &keys {
        toml += &format!("{} = {}\n", k, deps[*k]);
    }
    toml += "\n[[bin]]\nname = \"logyxprog\"\npath = \"src/main.rs\"\n\n[profile.release]\nopt-level = 3\n";
    std::fs::write(format!("{dir}/Cargo.toml"), toml)
        .map_err(|e| error::LogyxError::new(format!("{dir}/Cargo.toml: {e}")))?;
    std::fs::write(format!("{dir}/src/main.rs"), rust)
        .map_err(|e| error::LogyxError::new(format!("{dir}/src/main.rs: {e}")))?;
    let names: Vec<String> = keys.iter().map(|k| k.to_string()).collect();
    println!("// progetto cargo in {dir} (dipendenze: {})", names.join(", "));
    if !which("cargo") {
        println!("──── cargo non installato ────");
        return Ok(());
    }
    let comp = std::process::Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&dir)
        .output()
        .map_err(|e| error::LogyxError::new(format!("cargo: {e}")))?;
    if !comp.status.success() {
        return Err(error::LogyxError::new(format!(
            "cargo ha segnalato errori:\n{}",
            String::from_utf8_lossy(&comp.stderr)
        )));
    }
    println!("──── esecuzione del binario nativo ────");
    let run = std::process::Command::new(format!("{dir}/target/release/logyxprog"))
        .output()
        .map_err(|e| error::LogyxError::new(format!("binario: {e}")))?;
    print!("{}", String::from_utf8_lossy(&run.stdout));
    Ok(())
}

fn which(cmd: &str) -> bool {
    std::process::Command::new(cmd)
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
