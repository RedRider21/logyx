#!/usr/bin/env python3
# Copyright (C) 2026 Daniele Deplano (RedRider21)
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Esegue un programma Logyx col prototipo (lexer -> parser -> interprete).

Uso:
  python main.py <file.logyx>              esegue il programma (chiama main())
  python main.py render <file> <percorso>  stampa l'HTML reso da una route
  python main.py serve  <file> [porta]     avvia un server HTTP (default 8080)
  python main.py build  <file>             transpila in Rust (e compila se c'e' rustc)
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from logyx.interpreter import Interpreter
from logyx.modules import load_program
from logyx.errors import LogyxError


def _load(path):
    items = load_program(path)  # legge il file ed espande gli import
    interp = Interpreter()
    return interp, items


def cmd_run(path):
    interp, items = _load(path)
    interp.run(items)
    if interp.routes and "main" not in interp.globals.vars:
        print(
            f"(nessuna funzione main; il file definisce {len(interp.routes)} route: "
            f"prova  python main.py render {path} <percorso>  oppure  serve)",
            file=sys.stderr,
        )
    return 0


def cmd_render(path, route_path):
    interp, items = _load(path)
    interp.load(items)
    print(interp.render_route(route_path))
    return 0


def cmd_serve(path, port):
    from http.server import BaseHTTPRequestHandler, HTTPServer

    interp, items = _load(path)
    interp.load(items)

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            route_path = self.path.split("?", 1)[0]
            try:
                html = interp.render_route(route_path)
                body = html.encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            except LogyxError as e:
                body = f"404 - {e}".encode("utf-8")
                self.send_response(404)
                self.send_header("Content-Type", "text/plain; charset=utf-8")
                self.end_headers()
                self.wfile.write(body)

        def log_message(self, *args):
            pass

    server = HTTPServer(("127.0.0.1", port), Handler)
    rotte = ", ".join(sorted(interp.routes)) or "(nessuna)"
    print(f"Logyx in ascolto su http://127.0.0.1:{port}  route: {rotte}")
    print("Ctrl+C per fermare.")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nServer fermato.")
    return 0


def cmd_build(path):
    import shutil
    import subprocess
    from logyx.rustgen import RustTranspiler

    _, items = _load(path)
    rust = RustTranspiler().transpile(items)
    out_rs = os.path.splitext(path)[0] + ".rs"
    with open(out_rs, "w", encoding="utf-8") as f:
        f.write(rust)
    print(f"// Rust generato in {out_rs}\n")
    print(rust)
    if shutil.which("rustc"):
        binp = os.path.splitext(path)[0] + "_bin"
        comp = subprocess.run(["rustc", "-O", out_rs, "-o", binp], capture_output=True, text=True)
        if comp.returncode != 0:
            print("rustc ha segnalato errori:\n" + comp.stderr, file=sys.stderr)
            return 1
        print("──── esecuzione del binario nativo ────")
        run = subprocess.run([binp], capture_output=True, text=True)
        sys.stdout.write(run.stdout)
        return run.returncode
    print("──── rustc non installato ────")
    print(f"Per compilare dove Rust e' presente:  rustc -O {out_rs} -o app && ./app")
    return 0


def main(argv):
    if len(argv) < 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    try:
        if argv[1] == "render":
            if len(argv) < 4:
                print("uso: python main.py render <file> <percorso>", file=sys.stderr)
                return 2
            return cmd_render(argv[2], argv[3])
        if argv[1] == "serve":
            if len(argv) < 3:
                print("uso: python main.py serve <file> [porta]", file=sys.stderr)
                return 2
            port = int(argv[3]) if len(argv) > 3 and argv[3].isdigit() else 8080
            return cmd_serve(argv[2], port)
        if argv[1] == "build":
            if len(argv) < 3:
                print("uso: python main.py build <file>", file=sys.stderr)
                return 2
            return cmd_build(argv[2])
        return cmd_run(argv[1])
    except LogyxError as e:
        print(f"Errore: {e}", file=sys.stderr)
        return 1
    except OSError as e:
        print(f"Errore: {e}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
