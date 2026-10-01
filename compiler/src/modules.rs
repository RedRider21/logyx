// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Risoluzione dei moduli: espande gli `import` prima del backend.
//! Porting di `prototype/logyx/modules.py`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::ast::Item;
use crate::error::LogyxError;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn read_items(path: &Path) -> Result<Vec<Item>, LogyxError> {
    let src = std::fs::read_to_string(path)
        .map_err(|e| LogyxError::new(format!("{}: {e}", path.display())))?;
    let name = path.to_string_lossy().to_string();
    let toks = Lexer::new(&src, &name).tokenize()?;
    Parser::new(toks, name).parse()
}

fn resolve_path(spec: &str, base_dir: &Path) -> PathBuf {
    let mut cand = if Path::new(spec).is_absolute() {
        PathBuf::from(spec)
    } else {
        base_dir.join(spec)
    };
    if !cand.exists() && !spec.ends_with(".logyx") {
        let alt = PathBuf::from(format!("{}.logyx", cand.display()));
        if alt.exists() {
            cand = alt;
        }
    }
    cand
}

fn canon(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn expand(
    items: Vec<Item>,
    base_dir: &Path,
    seen: &mut HashSet<PathBuf>,
    out: &mut Vec<Item>,
    names: &mut HashSet<String>,
) -> Result<(), LogyxError> {
    for it in items {
        match it {
            Item::Import(spec) => {
                let path = resolve_path(&spec, base_dir);
                if !path.exists() {
                    return Err(LogyxError::new(format!(
                        "import: modulo non trovato: {:?}",
                        spec
                    )));
                }
                let c = canon(&path);
                if seen.contains(&c) {
                    continue;
                }
                seen.insert(c);
                let sub = read_items(&path)?;
                let parent = path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
                expand(sub, &parent, seen, out, names)?;
            }
            Item::Func(f) => {
                if names.contains(&f.name) {
                    return Err(LogyxError::new(format!(
                        "import: la funzione '{}' è definita più volte (conflitto tra moduli)",
                        f.name
                    )));
                }
                names.insert(f.name.clone());
                out.push(Item::Func(f));
            }
            other => out.push(other),
        }
    }
    Ok(())
}

/// Legge il file principale ed espande ricorsivamente i suoi import.
pub fn load_program(main_path: &str) -> Result<Vec<Item>, LogyxError> {
    let mp = PathBuf::from(main_path);
    let items = read_items(&mp)?;
    let mut seen = HashSet::new();
    seen.insert(canon(&mp));
    let base = mp.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    let mut out = Vec::new();
    let mut names = HashSet::new();
    expand(items, &base, &mut seen, &mut out, &mut names)?;
    Ok(out)
}
