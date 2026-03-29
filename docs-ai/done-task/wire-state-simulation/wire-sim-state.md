# WireSimState — 遅延ワイヤベースの状態管理

遅延ワイヤの値を中心に据えた状態表現。

## 概念

現行の `SimState` は全セルの値を `HashMap<Pos, bool>` で保持する。`WireSimState` は、tick 間で持ち越す必要がある情報のみを保持する:

1. **遅延ワイヤ値**: 後方ワイヤ（`dst < src`）のソースセルの前 tick の値
2. **入力なしセル値**: 入力ワイヤを持たないセルの前 tick の値（暗黙の自己遅延）

これらを統一的に「遅延スロット」として管理する。

## データ構造

```rust
/// 遅延スロットの種別。構築時に決定。
#[derive(Debug, Clone)]
enum DelayedSlot {
    /// 遅延ワイヤのソースセル値。wire_index はグローバルワイヤインデックス。
    Wire { wire_index: usize },
    /// 入力なしセルの値保持。cell_index は sorted_cells 内のインデックス。
    Cell { cell_index: usize },
}

/// 遅延ワイヤベースのシミュレーション状態。
#[derive(Debug, Clone)]
pub struct WireSimState {
    /// 遅延スロットの値。全て false で初期化。
    delayed_values: Vec<bool>,
    /// 各遅延スロットの種別。
    slots: Vec<DelayedSlot>,
    /// ワイヤインデックス → delayed_values のインデックス。
    /// 遅延ワイヤのみエントリを持つ。
    wire_to_slot: HashMap<usize, usize>,
    /// セルインデックス → delayed_values のインデックス。
    /// 入力なしセルのみエントリを持つ。
    cell_to_slot: HashMap<usize, usize>,
}
```

## 構築

```rust
impl WireSimState {
    pub fn from_circuit(circuit: &Circuit) -> Self {
        let mut slots = Vec::new();
        let mut wire_to_slot = HashMap::new();
        let mut cell_to_slot = HashMap::new();

        // 1. 遅延ワイヤのスロット割り当て
        for (i, wire) in circuit.wires().iter().enumerate() {
            if wire.dst < wire.src {
                let slot_index = slots.len();
                slots.push(DelayedSlot::Wire { wire_index: i });
                wire_to_slot.insert(i, slot_index);
            }
        }

        // 2. 入力なしセルのスロット割り当て
        for (cell_idx, pos) in circuit.sorted_cells().iter().enumerate() {
            let has_incoming = !circuit.incoming_indices(*pos).is_empty();
            let has_input = circuit.inputs().iter().any(|i| i.target() == *pos);
            if !has_incoming && !has_input {
                let slot_index = slots.len();
                slots.push(DelayedSlot::Cell { cell_index: cell_idx });
                cell_to_slot.insert(cell_idx, slot_index);
            }
        }

        let delayed_values = vec![false; slots.len()];
        Self { delayed_values, slots, wire_to_slot, cell_to_slot }
    }
}
```

## アクセス API

```rust
impl WireSimState {
    /// 遅延ワイヤの前 tick の値を取得する。
    pub fn get_delayed_wire(&self, wire_index: usize) -> Option<bool> {
        self.wire_to_slot.get(&wire_index).map(|&i| self.delayed_values[i])
    }

    /// 入力なしセルの前 tick の値を取得する。
    pub fn get_stateless_cell(&self, cell_index: usize) -> Option<bool> {
        self.cell_to_slot.get(&cell_index).map(|&i| self.delayed_values[i])
    }

    /// tick 完了時に遅延ワイヤの値を更新する。
    pub fn update_wire(&mut self, wire_index: usize, value: bool) {
        if let Some(&slot) = self.wire_to_slot.get(&wire_index) {
            self.delayed_values[slot] = value;
        }
    }

    /// tick 完了時に入力なしセルの値を更新する。
    pub fn update_cell(&mut self, cell_index: usize, value: bool) {
        if let Some(&slot) = self.cell_to_slot.get(&cell_index) {
            self.delayed_values[slot] = value;
        }
    }
}
```

## 現行 `SimState` との比較

| 項目 | `SimState` | `WireSimState` |
|------|-----------|---------------|
| 格納単位 | 全セル | 遅延ワイヤ + 入力なしセル |
| サイズ | `|cells|` | `|delayed_wires| + |stateless_cells|` |
| tick 間コピー | 全セルクローン | 不要（in-place 更新） |
| アクセス | `HashMap<Pos, bool>` O(1) 償却 | `Vec<bool>` O(1) |
| 初期化 | 全セル false | 全スロット false |

典型的な組み合わせ回路（前方ワイヤが大多数）では、`|delayed_wires|` は `|cells|` よりはるかに小さく、メモリ・コピーの両面で改善される。
