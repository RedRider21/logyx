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
        ("render", Some(p)) => {
            let rp = args.get(3).map(|s| s.as_str()).unwrap_or("/");
            run_render(p, rp)
        }
        _ => {
            eprintln!("uso: logyxc <tokens|parse|gen|build|build-wasm|render> <file.logyx> [percorso-route]");
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
    let exports = cg.wasm_exports(&names);
    if exports.is_empty() {
        return Err(error::LogyxError::new(
            "build-wasm: nessuna funzione esportabile a WASM \
             (firma fatta di int/float/bool/string)",
        ));
    }
    // wrapper esportati verso WASM. int/bool -> i32 (così JS usa Number, non BigInt);
    // float -> f64; string -> i32 (puntatore, ABI a memoria lineare, vedi design/web-wasm-fase1.md).
    let wasm_ret_ty = |t: &str| if t == "float" { "f64" } else { "i32" };
    let has_string =
        exports.iter().any(|e| e.ret == "string" || e.params.iter().any(|(_, t)| t == "string"));
    let mut wrappers = String::new();
    if has_string {
        // Helper della ABI stringhe: allocatore condiviso con JS + ricostruzione/ritorno di String.
        wrappers += "\
use std::alloc::{alloc as __ralloc, dealloc as __rdealloc, Layout};

#[export_name = \"__logyx_alloc\"]
pub extern \"C\" fn __logyx_alloc(len: i32) -> i32 {
    let n = (len.max(0) as usize).max(1);
    unsafe { __ralloc(Layout::from_size_align(n, 1).unwrap()) as i32 }
}

#[export_name = \"__logyx_free\"]
pub extern \"C\" fn __logyx_free(ptr: i32, len: i32) {
    let n = (len.max(0) as usize).max(1);
    unsafe { __rdealloc(ptr as *mut u8, Layout::from_size_align(n, 1).unwrap()); }
}

unsafe fn __logyx_take_str(ptr: i32, len: i32) -> String {
    let n = len.max(0) as usize;
    let v = std::slice::from_raw_parts(ptr as *const u8, n).to_vec();
    __rdealloc(ptr as *mut u8, Layout::from_size_align(n.max(1), 1).unwrap());
    String::from_utf8_lossy(&v).into_owned()
}

fn __logyx_ret_str(s: String) -> i32 {
    let b = s.into_bytes();
    let total = 4 + b.len();
    let layout = Layout::from_size_align(total.max(1), 1).unwrap();
    unsafe {
        let p = __ralloc(layout);
        let lb = (b.len() as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(lb.as_ptr(), p, 4);
        if !b.is_empty() { std::ptr::copy_nonoverlapping(b.as_ptr(), p.add(4), b.len()); }
        p as i32
    }
}

";
    }
    for e in &exports {
        let mut params = Vec::new();
        let mut prologue = String::new();
        let mut argvals = Vec::new();
        for (n, t) in &e.params {
            match t.as_str() {
                "string" => {
                    params.push(format!("{n}_ptr: i32, {n}_len: i32"));
                    prologue +=
                        &format!("    let {n} = unsafe {{ __logyx_take_str({n}_ptr, {n}_len) }};\n");
                    argvals.push(n.clone());
                }
                "int" => {
                    params.push(format!("{n}: i32"));
                    argvals.push(format!("{n} as i64"));
                }
                "bool" => {
                    params.push(format!("{n}: i32"));
                    argvals.push(format!("({n} != 0)"));
                }
                _ => {
                    params.push(format!("{n}: f64")); // float
                    argvals.push(n.clone());
                }
            }
        }
        let call = format!("{}({})", e.name, argvals.join(", "));
        let body = match e.ret.as_str() {
            "string" => format!("__logyx_ret_str({call})"),
            "int" | "bool" => format!("({call}) as i32"),
            _ => call, // float
        };
        wrappers += &format!(
            "#[export_name = \"{}\"]\npub extern \"C\" fn __wasm_{}({}) -> {} {{\n{}    {}\n}}\n\n",
            e.name,
            e.name,
            params.join(", "),
            wasm_ret_ty(&e.ret),
            prologue,
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
    std::fs::write(&html_out, wasm_demo_html(&wasm_name, &exports))
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

fn wasm_demo_html(wasm_name: &str, exports: &[codegen::ExportSig]) -> String {
    // Descrittore JS delle firme: guida sia la UI sia il marshalling degli argomenti.
    let sigs = exports
        .iter()
        .map(|e| {
            let ps = e
                .params
                .iter()
                .map(|(_, t)| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{name:\"{}\",params:[{}],ret:\"{}\"}}", e.name, ps, e.ret)
        })
        .collect::<Vec<_>>()
        .join(",\n    ");
    format!(
        r#"<!doctype html>
<html lang="it">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Logyx → WebAssembly</title>
<style>
  body{{font-family:-apple-system,Segoe UI,Roboto,sans-serif; max-width:680px; margin:40px auto; padding:0 20px; color:#171a2b}}
  h1{{letter-spacing:-.02em}}
  .fn{{border:1px solid #e3e5ef; border-radius:12px; padding:14px 16px; margin:12px 0}}
  code{{font-family:ui-monospace,monospace; color:#5b4be1; font-weight:600}}
  input{{padding:4px 6px; margin:0 2px}}
  input[type=number]{{width:70px}}
  input[type=text]{{width:170px}}
  button{{margin-left:8px; padding:5px 12px; border-radius:8px; border:1px solid #5b4be1; background:#5b4be1; color:#fff; cursor:pointer}}
  .res{{font-weight:700; color:#5b4be1; margin-left:8px; word-break:break-word}}
</style>
</head>
<body>
<h1>Logyx → WebAssembly</h1>
<p>Funzioni di Logyx compilate a WASM e invocate direttamente nel browser. Le stringhe passano il
confine JS↔WASM tramite la memoria lineare del modulo (ABI minimale, nessun <code>wasm-bindgen</code>).</p>
<div id="app">caricamento…</div>
<script>
const SIGS = [
    {sigs}
];
const enc = new TextEncoder(), dec = new TextDecoder();
let ex;
function callFn(sig, raw) {{
  const args = [];
  for (let i = 0; i < sig.params.length; i++) {{
    const t = sig.params[i];
    if (t === "string") {{
      const bytes = enc.encode(raw[i] != null ? String(raw[i]) : "");
      const ptr = ex.__logyx_alloc(bytes.length);
      new Uint8Array(ex.memory.buffer, ptr, bytes.length).set(bytes);
      args.push(ptr, bytes.length);
    }} else if (t === "bool") {{
      args.push(Number(raw[i]) ? 1 : 0);
    }} else {{
      args.push(Number(raw[i]));
    }}
  }}
  const r = ex[sig.name](...args);
  if (sig.ret === "string") {{
    const m = new Uint8Array(ex.memory.buffer);
    const len = m[r] | (m[r + 1] << 8) | (m[r + 2] << 16) | (m[r + 3] * 16777216);
    const out = dec.decode(m.subarray(r + 4, r + 4 + len));
    ex.__logyx_free(r, 4 + len);
    return out;
  }}
  if (sig.ret === "bool") return (r !== 0);
  return r;
}}
(async () => {{
  const app = document.getElementById("app");
  try {{
    const bytes = await (await fetch("{wasm}")).arrayBuffer();
    const {{ instance }} = await WebAssembly.instantiate(bytes, {{}});
    ex = instance.exports;
    app.innerHTML = "";
    for (const sig of SIGS) {{
      const div = document.createElement("div"); div.className = "fn";
      const lab = document.createElement("span"); lab.innerHTML = "<code>" + sig.name + "</code> ( ";
      div.appendChild(lab);
      const inputs = [];
      sig.params.forEach((t, i) => {{
        const inp = document.createElement("input");
        if (t === "string") {{ inp.type = "text"; inp.placeholder = "testo"; }}
        else {{ inp.type = "number"; inp.value = "0"; }}
        inputs.push(inp); div.appendChild(inp);
        if (i < sig.params.length - 1) div.appendChild(document.createTextNode(", "));
      }});
      div.appendChild(document.createTextNode(" ) "));
      const btn = document.createElement("button"); btn.textContent = "esegui";
      const res = document.createElement("span"); res.className = "res";
      btn.onclick = () => {{
        try {{ res.textContent = "= " + callFn(sig, inputs.map(x => x.value)); }}
        catch (e) {{ res.textContent = "errore: " + e; }}
      }};
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
        wasm = wasm_name,
        sigs = sigs
    )
}

/// Web Fase 2a: compila il file (route + render) a un binario server nativo e ne
/// stampa l'HTML reso per il percorso `route_path` (come `python main.py render`).
fn run_render(path: &str, route_path: &str) -> Result<(), error::LogyxError> {
    let items = modules::load_program(path)?;
    let mut cg = codegen::Codegen::new();
    let rust = cg.generate(&items)?;
    if !cg.deps.is_empty() {
        return Err(error::LogyxError::new(
            "render: i file con dipendenze da crate ('use rust') non sono ancora supportati",
        ));
    }
    if !which("rustc") {
        return Err(error::LogyxError::new("rustc non installato"));
    }
    let out_rs = change_ext(path, "rs");
    std::fs::write(&out_rs, &rust).map_err(|e| error::LogyxError::new(format!("{out_rs}: {e}")))?;
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
    let run = std::process::Command::new(&bin)
        .arg(route_path)
        .output()
        .map_err(|e| error::LogyxError::new(format!("{bin}: {e}")))?;
    print!("{}", String::from_utf8_lossy(&run.stdout));
    if !run.status.success() {
        eprint!("{}", String::from_utf8_lossy(&run.stderr));
        return Err(error::LogyxError::new("render: route non trovata"));
    }
    Ok(())
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
