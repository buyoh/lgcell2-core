# 出力形式 — AllCell / ViewPort

tick 完了時に保持するセル状態の出力形式を2種類提供し、利用シーンに応じた効率化を実現する。

## 背景・動機

現行の `TickSnapshot` は全セルの値 `Vec<(Pos, bool)>` を保持する。CLI のバッチ実行や JSON 出力では全セルが必要だが、Web UI 上のリアルタイム表示では画面内のセルのみが必要である。

大規模回路でセル数が多い場合、毎 tick 全セルの `HashMap<Pos, bool>` を構築するのはコストが高い。ViewPort 形式を導入することで、表示範囲内のセルのみを収集し、メモリ割り当てとイテレーションのコストを削減する。

## 設計

### データ型

```rust
/// 矩形領域（含む-含む）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rect {
    pub min: Pos,
    pub max: Pos,
}

impl Rect {
    pub fn new(min: Pos, max: Pos) -> Self {
        Self { min, max }
    }

    pub fn contains(&self, pos: Pos) -> bool {
        pos.x >= self.min.x && pos.x <= self.max.x
            && pos.y >= self.min.y && pos.y <= self.max.y
    }
}
```

### 出力形式

```rust
/// tick 完了時の出力形式。
#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// すべてのセルの状態を収集する。
    AllCell,
    /// 指定された矩形領域内のセルのみ収集する。
    ViewPort(Vec<Rect>),
}
```

### tick 出力

```rust
/// tick 完了時に収集されるセル状態。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickOutput {
    pub tick: u64,
    pub cells: HashMap<Pos, bool>,
}
```

`TickOutput` は `TickSnapshot` を置き換える。`cells` の型を `Vec<(Pos, bool)>` から `HashMap<Pos, bool>` に変更し、ランダムアクセスを可能にする。

### WireSimulator への統合

`WireSimulator` は `OutputFormat` を保持し、tick 完了時に出力を構築する:

```rust
pub struct WireSimulator {
    // ... 既存フィールド ...
    output_format: OutputFormat,
    last_output: Option<TickOutput>,
}
```

#### 出力の構築（tick 完了時）

```rust
fn build_output(&self) -> TickOutput {
    let cells = match &self.output_format {
        OutputFormat::AllCell => {
            self.circuit.sorted_cells().iter().enumerate()
                .map(|(idx, &pos)| (pos, self.cell_values[idx]))
                .collect()
        }
        OutputFormat::ViewPort(rects) => {
            self.circuit.sorted_cells().iter().enumerate()
                .filter(|(_, pos)| rects.iter().any(|r| r.contains(**pos)))
                .map(|(idx, &pos)| (pos, self.cell_values[idx]))
                .collect()
        }
    };
    TickOutput { tick: self.tick, cells }
}
```

#### 出力形式の変更

```rust
impl WireSimulator {
    /// 出力形式を変更する。次の tick 完了から反映される。
    pub fn set_output_format(&mut self, format: OutputFormat) {
        self.output_format = format;
    }
}
```

### ViewPort の最適化可能性

`sorted_cells` は `Pos` の辞書順 `(x, y)` でソートされている。ViewPort フィルタリングで全セルを線形走査する代わりに、x 座標の範囲で二分探索して走査範囲を絞ることが可能。ただし初期実装では線形走査で十分であり、プロファイル結果に応じて最適化する。

### 利用シーン

| 形式 | 利用シーン | 例 |
|------|-----------|-----|
| AllCell | バッチ実行、JSON 出力、テスト | CLI `run`, `io::json` |
| ViewPort | リアルタイム表示 | Web UI, CLI ビューモード |

### コンストラクタ

```rust
impl WireSimulator {
    /// AllCell 形式でシミュレータを構築する。
    pub fn new(circuit: Circuit) -> Self {
        Self::with_output_format(circuit, OutputFormat::AllCell)
    }

    /// 出力形式を指定してシミュレータを構築する。
    pub fn with_output_format(circuit: Circuit, output_format: OutputFormat) -> Self {
        // ...
    }
}
```

## TickSnapshot との関係

現行の `TickSnapshot` は `Vec<(Pos, bool)>` を保持するが、新しい `TickOutput` は `HashMap<Pos, bool>` を保持する。

- `run_with_snapshots()` は `Vec<TickOutput>` を返すように変更する
- ViewPort 形式では、各 tick の出力がビューポート内のセルのみを含む

## view モジュールとの関係

現行の `ViewRenderer::render_grid()` は `&SimState` を受け取り、ビューポート範囲のセルを 1 つずつ `state.get(pos)` で参照している。

移行方針: `render_grid()` / `render_frame()` の引数を `&SimState` から `&HashMap<Pos, bool>` に変更する。`HashMap::get(&pos)` は `SimState::get(pos)` と同じパターンでアクセスできるため、内部ロジックの変更は最小限。

`bin/lgcell2/view.rs` では `simulator.state()` の代わりに、`WireSimulator` から ViewPort 形式の `TickOutput` を取得し、その `cells` フィールドを渡す形にする。
