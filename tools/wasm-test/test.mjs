import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(__dirname, "../../pkg");

const wasmPath = join(pkgDir, "lgcell2_core_bg.wasm");
const wasmBytes = await readFile(wasmPath);

const bg = await import("../../pkg/lgcell2_core_bg.js");

const wasmModule = await WebAssembly.compile(wasmBytes);
const imports = { "./lgcell2_core_bg.js": bg };
const wasmInstance = await WebAssembly.instantiate(wasmModule, imports);

bg.__wbg_set_wasm(wasmInstance.exports);

if (wasmInstance.exports.__wbindgen_start) {
  wasmInstance.exports.__wbindgen_start();
}

const { simulate } = bg;

{
  const circuit = JSON.stringify({
    wires: [{ src: [0, 0], dst: [1, 0], kind: "positive" }],
  });

  const result = simulate(circuit, 3n);
  const parsed = JSON.parse(result);

  assert.ok(Array.isArray(parsed.ticks), "ticks is array");
  assert.equal(parsed.ticks.length, 3, "3 ticks");
  console.log("PASS: basic simulation");
}

{
  try {
    simulate("invalid json", 1n);
    assert.fail("should throw on invalid json");
  } catch (_e) {
    assert.ok(true, "error thrown for invalid json");
  }

  console.log("PASS: invalid json error");
}

{
  const circuit = JSON.stringify({
    wires: [{ src: [0, 0], dst: [1, 0], kind: "unknown" }],
  });

  try {
    simulate(circuit, 1n);
    assert.fail("should throw on invalid wire kind");
  } catch (_e) {
    assert.ok(true, "error thrown for invalid wire kind");
  }

  console.log("PASS: invalid wire kind error");
}

console.log("All WASM tests passed.");
