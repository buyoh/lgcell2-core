# フェーズ 4: Node.js 動作確認テスト

作成日: 2026-03-24
ステータス: 未着手

## 概要

ビルドされた WASM パッケージが正しく動作するか、Node.js 上で簡易な動作確認テストを実行する。

## 設計

### テストスクリプト

nospace20 の `tools/wasm-test/test.mjs` パターンに倣い、`tools/wasm-test/test.mjs` を作成する。

`--target bundler` で出力された WASM パッケージは、`import ... from "*.wasm"` 形式のインポートを使用するため、Node.js から直接利用する場合は手動で WASM ランタイムを初期化する必要がある。

```javascript
// tools/wasm-test/test.mjs
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const pkgDir = join(__dirname, "../../pkg");

// WASM ファイルを読み込んで初期化 (bundler target 対応)
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

// テスト 1: 基本的なシミュレーション実行
{
  const circuit = JSON.stringify({
    wires: [
      { src: [0, 0], dst: [1, 0], kind: "positive" },
    ],
  });
  const result = simulate(circuit, 3);
  const parsed = JSON.parse(result);
  assert.ok(Array.isArray(parsed.ticks), "ticks is array");
  assert.equal(parsed.ticks.length, 3, "3 ticks");
  console.log("PASS: basic simulation");
}

// テスト 2: 不正な JSON でエラーが返ること
{
  try {
    simulate("invalid json", 1);
    assert.fail("should throw on invalid json");
  } catch (e) {
    assert.ok(true, "error thrown for invalid json");
  }
  console.log("PASS: invalid json error");
}

// テスト 3: 不正なワイヤ種別でエラーが返ること
{
  const circuit = JSON.stringify({
    wires: [
      { src: [0, 0], dst: [1, 0], kind: "unknown" },
    ],
  });
  try {
    simulate(circuit, 1);
    assert.fail("should throw on invalid wire kind");
  } catch (e) {
    assert.ok(true, "error thrown for invalid wire kind");
  }
  console.log("PASS: invalid wire kind error");
}

console.log("All WASM tests passed.");
```

### ファイル名の注意

wasm-pack の出力ファイル名はパッケージ名のハイフンをアンダースコアに変換する。`lgcell2-core` → `lgcell2_core_bg.wasm`, `lgcell2_core_bg.js` となる。

### 実行方法

```bash
# WASM のビルド（フェーズ 3）の後に実行
node tools/wasm-test/test.mjs
```

### ディレクトリ構成

```
tools/
  wasm-test/
    test.mjs        # WASM 動作確認テスト
```

package.json は作成しない（`node:assert` 等の Node.js 組み込みモジュールのみ使用）。

## ステップ

1. `tools/wasm-test/` ディレクトリを作成
2. `test.mjs` を作成（上記コード。テスト対象の回路 JSON は既存テストリソースから適切なものを選択するか、最小限のものをインライン定義）
3. `./build-wasm.sh` で WASM をビルド
4. `node tools/wasm-test/test.mjs` で全テストが PASS することを確認

## 備考

- このテストは CI ではなく手動実行を想定。CI 統合は将来の課題
- テストケースは最小限に留め、シミュレーションロジック自体のテストは Rust 側のユニットテストに委ねる
- WASM 初期化コードは bundler target 特有のボイラープレート。`--target nodejs` にすれば不要だが、ブラウザ互換性を優先する
