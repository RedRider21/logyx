#!/usr/bin/env python3
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Genera docs/manuale.html dal MANUAL.md (unica fonte di verità).

Uso:  python3 tools/build_manual.py
Richiede: python-markdown.
"""

import os
import markdown

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

TEMPLATE = """<!doctype html>
<html lang="it">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Manuale di Logyx</title>
<meta name="description" content="Manuale di riferimento del linguaggio Logyx: sintassi, costrutti, tipi, funzioni builtin e compilazione nativa.">
<link rel="icon" href="favicon.ico" sizes="any">
<link rel="icon" href="logo.svg" type="image/svg+xml">
<style>
  :root{{
    --bg:#f6f7fb; --panel:#ffffff; --text:#171a2b; --muted:#5a6076; --border:#e3e5ef;
    --accent:#5b4be1; --code-bg:#0f1220; --code-text:#e7e9f5; --shadow:0 12px 40px rgba(23,26,43,.10);
  }}
  :root[data-theme="dark"]{{
    --bg:#0c0e17; --panel:#141826; --text:#e8eaf3; --muted:#9aa3bd; --border:#252b3f;
    --accent:#8b7dff; --code-bg:#0a0c15; --code-text:#e7e9f5; --shadow:0 14px 44px rgba(0,0,0,.45);
  }}
  @media (prefers-color-scheme:dark){{
    :root:not([data-theme="light"]){{
      --bg:#0c0e17; --panel:#141826; --text:#e8eaf3; --muted:#9aa3bd; --border:#252b3f;
      --accent:#8b7dff; --code-bg:#0a0c15; --code-text:#e7e9f5; --shadow:0 14px 44px rgba(0,0,0,.45);
    }}
  }}
  *{{box-sizing:border-box}}
  html{{scroll-behavior:smooth}}
  body{{margin:0; background:var(--bg); color:var(--text);
    font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,Helvetica,Arial,sans-serif;
    line-height:1.65; -webkit-font-smoothing:antialiased}}
  a{{color:var(--accent); text-decoration:none}} a:hover{{text-decoration:underline}}
  header{{position:sticky; top:0; z-index:10; backdrop-filter:blur(10px);
    background:color-mix(in srgb, var(--bg) 82%, transparent); border-bottom:1px solid var(--border)}}
  .nav{{max-width:860px; margin:0 auto; display:flex; align-items:center; gap:12px; padding:12px 20px}}
  .brand{{display:flex; align-items:center; gap:10px; font-weight:700; font-size:1.15rem}}
  .brand svg{{width:32px; height:32px; display:block}}
  .nav .spacer{{flex:1}}
  .ctrl{{display:inline-flex; gap:6px; background:var(--panel); border:1px solid var(--border);
    border-radius:10px; padding:4px}}
  .ctrl button{{all:unset; cursor:pointer; padding:5px 9px; border-radius:7px; font-size:.82rem;
    font-weight:600; color:var(--muted)}}
  .ctrl button.active{{background:color-mix(in srgb,var(--accent) 16%,transparent); color:var(--accent)}}
  main{{max-width:860px; margin:0 auto; padding:28px 20px 72px}}
  h1{{font-size:2rem; letter-spacing:-.02em; margin:.2em 0 .5em}}
  h2{{font-size:1.5rem; margin:1.7em 0 .5em; padding-top:.3em; border-top:1px solid var(--border)}}
  h3{{font-size:1.15rem; margin:1.4em 0 .4em}}
  p,li{{color:var(--text)}}
  code{{font-family:ui-monospace,SFMono-Regular,Menlo,Consolas,monospace; font-size:.9em;
    background:color-mix(in srgb,var(--accent) 12%,transparent); color:var(--accent);
    padding:1px 6px; border-radius:6px}}
  pre{{background:var(--code-bg); border:1px solid var(--border); border-radius:12px; padding:16px 18px;
    overflow:auto; box-shadow:var(--shadow)}}
  pre code{{background:none; color:var(--code-text); padding:0; font-size:.88rem; line-height:1.6}}
  table{{width:100%; border-collapse:collapse; margin:1em 0; font-size:.94rem}}
  th,td{{text-align:left; padding:9px 12px; border-bottom:1px solid var(--border); vertical-align:top}}
  th{{color:var(--muted); font-weight:700}}
  blockquote{{margin:1em 0; padding:8px 16px; border-left:4px solid var(--accent);
    background:color-mix(in srgb,var(--accent) 8%,transparent); border-radius:0 10px 10px 0; color:var(--muted)}}
  hr{{border:none; border-top:1px solid var(--border); margin:2em 0}}
  .backlink{{display:inline-block; margin-bottom:8px; color:var(--muted); font-weight:600; font-size:.92rem}}
</style>
</head>
<body>
<header>
  <div class="nav">
    <a class="brand" href="index.html" aria-label="Logyx">
      <svg viewBox="0 0 256 256" aria-hidden="true"><circle cx="128" cy="128" r="120" fill="#5b4be1"/><path d="M494 1020 432 1188Q404 1265 366.5 1293.0Q329 1321 235 1321H123V1556H330Q512 1556 610.5 1479.0Q709 1402 777 1220L1235 0H877L654 588L419 0H61Z" transform="translate(64.70,216.00) scale(0.0977,-0.0977)" fill="#fff"/><circle cx="128" cy="44" r="15" fill="#22c3dd"/></svg>
      <span>Logyx</span>
    </a>
    <div class="spacer"></div>
    <div class="ctrl" role="group" aria-label="tema">
      <button data-theme-btn="light" title="chiaro">☀</button>
      <button data-theme-btn="dark" title="scuro">☾</button>
    </div>
  </div>
</header>
<main>
  <a class="backlink" href="index.html">← Logyx</a>
{body}
</main>
<script>
(function(){{
  function store(k,v){{try{{localStorage.setItem(k,v);}}catch(e){{}}}}
  function load(k){{try{{return localStorage.getItem(k);}}catch(e){{return null;}}}}
  function applyTheme(mode){{
    if(mode==="light"||mode==="dark") document.documentElement.setAttribute("data-theme",mode);
    else document.documentElement.removeAttribute("data-theme");
    document.querySelectorAll("[data-theme-btn]").forEach(function(b){{
      b.classList.toggle("active", b.getAttribute("data-theme-btn")===mode);
    }});
    store("logyx-theme",mode||"");
  }}
  document.querySelectorAll("[data-theme-btn]").forEach(function(b){{
    b.addEventListener("click",function(){{applyTheme(b.getAttribute("data-theme-btn"));}});
  }});
  applyTheme(load("logyx-theme")||"");
}})();
</script>
</body>
</html>
"""


def main():
    src = open(os.path.join(ROOT, "MANUAL.md"), encoding="utf-8").read()
    body = markdown.markdown(src, extensions=["tables", "fenced_code", "toc"])
    html = TEMPLATE.format(body=body)
    out = os.path.join(ROOT, "docs", "manuale.html")
    open(out, "w", encoding="utf-8").write(html)
    print(f"scritto {out} ({len(html)} byte)")


if __name__ == "__main__":
    main()
