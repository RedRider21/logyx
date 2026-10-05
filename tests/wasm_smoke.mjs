// Copyright (C) 2026 Daniele Deplano (RedRider21)
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Smoke test del client WASM (web -> WASM, Fasi 0/1): esegue il COLLANTE JS REALE
// della pagina generata da `logyxc build-wasm` (non una sua reimplementazione),
// con un DOM simulato minimale, e verifica le chiamate oltre il confine JS<->WASM
// (numeri e stringhe via memoria lineare). Si lancia da tests/wasm_smoke.sh.

import fs from "fs";

const ROOT = new URL("..", import.meta.url).pathname;
const EXAMPLES = ROOT + "examples/";

// Casi attesi per ciascuna pagina (nome funzione -> [argomenti, risultato atteso]).
const SUITES = {
  native_wasm: [
    ["raddoppia", [21], 42],
    ["somma", [3, 4], 7],
    ["fattoriale", [5], 120],
    ["pari", [4], true],
    ["pari", [3], false],
  ],
  native_webstr: [
    ["saluta", ["mondo"], "Ciao mondo"],
    ["grida", ["logyx"], "LOGYX"],
    ["lunghezza", ["ciao"], 4],
    ["vuoto", [""], true],
    ["grado", ["ciao", 2], "CIAO"],
    ["ripeti", ["ab", 3], "ababab"],
    ["etichetta", ["Ada", 36], "Ada (36)"],
    ["saluta", ["èà€"], "Ciao èà€"], // UTF-8 accentato
  ],
};

// DOM/fetch minimali: bastano a far girare l'IIFE della pagina senza un browser.
function installShims() {
  const mkEl = () =>
    new Proxy(
      { children: [], appendChild() {}, },
      { get: (t, k) => (k in t ? t[k] : undefined), set: (t, k, v) => ((t[k] = v), true) }
    );
  globalThis.document = {
    getElementById: () => mkEl(),
    createElement: () => mkEl(),
    createTextNode: () => ({}),
  };
  globalThis.fetch = async (p) => ({
    arrayBuffer: async () => fs.readFileSync(EXAMPLES + p),
  });
}

// Esegue lo <script> della pagina e restituisce callFn/SIGS/ex reali.
async function loadPage(base) {
  const html = fs.readFileSync(EXAMPLES + base + ".html", "utf8");
  const script = html.match(/<script>([\s\S]*?)<\/script>/)[1];
  const bag = {};
  const wrapped =
    script + "\n; globalThis.__bag.callFn = callFn; globalThis.__bag.SIGS = SIGS; globalThis.__bag.getEx = () => ex;";
  globalThis.__bag = bag;
  (0, eval)(wrapped);
  await new Promise((r) => setTimeout(r, 200)); // attende fetch + WebAssembly.instantiate
  if (!bag.getEx()) throw new Error(`${base}: WASM non istanziato dalla pagina`);
  return bag;
}

installShims();
let ok = 0,
  tot = 0;
for (const [base, cases] of Object.entries(SUITES)) {
  const bag = await loadPage(base);
  const sig = (n) => bag.SIGS.find((s) => s.name === n);
  console.log(`== ${base} ==`);
  for (const [name, args, exp] of cases) {
    tot++;
    const got = bag.callFn(sig(name), args);
    const pass = String(got) === String(exp);
    if (pass) ok++;
    console.log(
      `  ${pass ? "OK  " : "FAIL"} ${name}(${args.map((a) => JSON.stringify(a)).join(",")}) = ${JSON.stringify(got)}` +
        (pass ? "" : `  atteso ${JSON.stringify(exp)}`)
    );
  }
}
console.log(`\n${ok}/${tot} chiamate dal collante reale delle pagine`);
process.exit(ok === tot ? 0 : 1);
