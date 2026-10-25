#!/usr/bin/env python3
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Verifica gli esempi del riferimento (docs/riferimento.html) nei tre backend.

Ogni esempio eseguibile viene avvolto in un programma (`fn main` se manca), eseguito
con l'interprete, il transpiler Python→Rust e il compilatore `logyxc`, e confrontato:
i tre output devono coincidere, non essere errori, e — se la voce dichiara un output
(`out`) — essere uguali a quello. Le voci che richiedono altro (server, browser,
crate, terminale, più file, tastiera) sono escluse e verificate dagli altri test.
Esce con 0 se tutto conforme, 1 altrimenti.
"""
import json, os, re, shutil, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
LOGYXC = os.path.join(ROOT, "compiler/target/release/logyxc")
CATS = {"basics", "types", "ops", "flow", "fn", "err", "data", "str", "num", "coll", "io", "json"}
# voci non eseguibili da sole, con il motivo
ESCLUSE = {
    "nil": "solo interprete (i backend tipizzati lo rifiutano)",
    "read_line / read_all": "legge da tastiera",
    "random": "valore casuale",
    "now": "dipende dall'orologio",
    "args": "dipende dalla riga di comando",
    "env": "dipende dall'ambiente",
}


def voci():
    html = open(os.path.join(ROOT, "docs/riferimento.html"), encoding="utf-8").read()
    js = "const fs=require('fs');const h=fs.readFileSync(process.argv[1],'utf8');" \
         "const s=h.indexOf('var E=[');const a=h.indexOf('[',s);let d=0,j=a,inS=false,q='';" \
         "for(;j<h.length;j++){const c=h[j];if(inS){if(c=='\\\\'){j++;continue}if(c==q)inS=false;continue}" \
         "if(c=='\"'||c==\"'\"){inS=true;q=c;continue}if(c=='[')d++;else if(c==']'){d--;if(!d)break}}" \
         "console.log(JSON.stringify(eval(h.slice(a,j+1))))"
    out = subprocess.run(["node", "-e", js, os.path.join(ROOT, "docs/riferimento.html")],
                         capture_output=True, text=True, check=True).stdout
    un = lambda t: t.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
    return [dict(cat=e["cat"], name=e["name"], code=un(e.get("code", "")),
                 out=un(e["out"]) if e.get("out") else None) for e in json.loads(out)]


def programma(code):
    if re.search(r"^fn main\(", code, re.M):
        return code
    righe, top, body, i = code.split("\n"), [], [], 0
    while i < len(righe):
        r = righe[i]
        if re.match(r"^(fn|record|enum|import|use|extern|const)\b", r):
            blocco, prof = [r], r.count("{") - r.count("}")
            while prof > 0 and i + 1 < len(righe):
                i += 1
                blocco.append(righe[i])
                prof += righe[i].count("{") - righe[i].count("}")
            top.append("\n".join(blocco))
        else:
            body.append(r)
        i += 1
    return "\n\n".join(top + ["fn main() {\n" + "\n".join("    " + b for b in body) + "\n}"]) + "\n"


def esegui(cmd, cwd):
    try:
        p = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, timeout=120,
                           stdin=subprocess.DEVNULL)
        return (p.stdout + p.stderr).strip()
    except subprocess.TimeoutExpired:
        return "TIMEOUT"


def nativo(out):
    parti = out.split("──── esecuzione del binario nativo ────")
    return parti[1].strip() if len(parti) > 1 else "NON COMPILA: " + out[-400:]


def main():
    tmp = tempfile.mkdtemp(prefix="logyx_rif_")
    ok = ko = salt = 0
    try:
        for n, v in enumerate(voci()):
            if v["cat"] not in CATS or not v["code"] or v["name"] in ESCLUSE:
                salt += 1
                continue
            f = os.path.join(tmp, f"v{n:03d}.logyx")
            open(f, "w", encoding="utf-8").write(programma(v["code"]))
            # tutto gira nella cartella temporanea: i file scritti dagli esempi restano lì
            main_py = os.path.join(ROOT, "prototype", "main.py")
            it = esegui(["python3", main_py, f], tmp)
            py = nativo(esegui(["python3", main_py, "build", f], tmp))
            rs = nativo(esegui([LOGYXC, "build", f], tmp))
            errore = any(x.startswith(("Errore", "NON COMPILA", "TIMEOUT")) for x in (it, py, rs))
            atteso_ok = v["out"] is None or it == v["out"].strip()
            if it == py == rs and not errore and atteso_ok:
                ok += 1
                print(f"  OK    {v['name']}")
            else:
                ko += 1
                print(f"  DIFF  {v['name']}")
                if v["out"] is not None and not atteso_ok:
                    print("    dichiarato:  " + v["out"].replace("\n", " | "))
                print("    interprete:  " + it.replace("\n", " | ")[:300])
                print("    transpiler:  " + py.replace("\n", " | ")[:300])
                print("    compilatore: " + rs.replace("\n", " | ")[:300])
    finally:
        shutil.rmtree(tmp, ignore_errors=True)
    print("---------------------------------------")
    print(f"Esempi conformi: {ok}   Differenti: {ko}   (esclusi: {salt}, vedi ESCLUSE e categorie)")
    print("RIFERIMENTO CONFORME ✅" if ko == 0 else "RIFERIMENTO NON CONFORME")
    return 0 if ko == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
