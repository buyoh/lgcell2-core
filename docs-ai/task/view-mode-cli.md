# `--view`: TUI ビューモード

回路シミュレーションの状態を端末全画面にアスキーアートでリアルタイム表示する CLI モード。

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

既存の `lgcell2` CLI はバッチ処理で JSON 結果を出力するのみであり、シミュレーションの進行を視覚的に確認する手段がない。`--view` フラグで TUI（Terminal User Interface）モードを提供し、回路の状態遷移をリアルタイムに観察可能にする。

## 設計・方針

### CLI 引数

```
lgcell2 --view <file>
lgcell2 --view  (標準入力から JSON 読み込み後にビューモードへ遷移)
```

- `--view` (`-v`): ビューモードで起動する
- `--ticks` と `--view` は排他。両方指定時はエラー
- `--interactive` と `--view` も排他

### 画面構成

```
..........##.._.._..
..........##.._.._..
.._##_....##.._.._..
.._##_..............
....................
tick:42 | running               (q:quit space:pause arrows:scroll)
```

- 端末全体をグリッド表示に使用する
- 最下行（1行）はステータスバーとして使用
- 表示可能領域 = 端末の行数 - 1 行（ステータスバー分）

### セル表示ルール

| 状態 | 文字 |
|------|------|
| ON (`true`) | `#` |
| OFF (`false`) | `_` |
| セルなし | `.` |

### ステータスバー

最下行に以下の形式で表示する。全てのメッセージは英語とする。

```
tick:<現在のtick> | <running/paused>               (q:quit space:pause arrows:scroll)
```

- 左寄せで tick 番号と状態を表示
- 右寄せで操作ヘルプを表示

### キー操作

| キー | 動作 |
|------|------|
| `q` | ビューモード終了。端末をリストアして正常終了 |
| `Space` | 一時停止 / 再開のトグル |
| `↑` | ビューポートを上にスクロール（1行） |
| `↓` | ビューポートを下にスクロール（1行） |
| `←` | ビューポートを左にスクロール（1列） |
| `→` | ビューポートを右にスクロール（1列） |

### ビューポートとスクロール

- 初期ビューポート位置: 回路のバウンディングボックスの左上角
- バウンディングボックス: 全セルの座標から `(min_x, min_y)` 〜 `(max_x, max_y)` を算出
- スクロールはバウンディングボックスの範囲外も許容する（範囲外は `.` で表示）

### シミュレーション実行

- 1 tick = 0.2 秒間隔で自動進行
- `Simulator::tick()` を使用し、1 tick ずつ進める
- 一時停止中は tick を進行せず、画面は最後の状態を維持
- シミュレーション開始前（tick 0）の状態も表示する

### イベントループ

```rust
loop {
    // 次の tick までの残り時間（paused なら無制限待機）
    let timeout = if paused { None } else { Some(remaining) };

    if poll(timeout) {
        match read_event() {
            Key('q') => break,
            Key(Space) => paused = !paused,
            Key(Arrow) => update_viewport(),
            _ => {}
        }
    }

    if !paused && elapsed >= TICK_INTERVAL {
        simulator.tick();
        render();
        reset_timer();
    }
}
```

### 依存ライブラリ

- **`crossterm`**: クロスプラットフォーム端末操作（raw モード、キー入力、画面クリア、カーソル制御）
- `Cargo.toml` の `[dependencies]` に追加。将来の feature flag 分離（`separate-clap-dependency` タスク）では `cli` feature にまとめる

### モジュール構成

ビューモードは CLI 固有の機能のため、バイナリ crate 内に配置する。

```
src/bin/lgcell2/
    main.rs       # --view フラグ追加、モード分岐
    view.rs       # ビューモードの実装
```

`view.rs` の主要構造:

```rust
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
    cursor, execute,
};

const TICK_INTERVAL: Duration = Duration::from_millis(200);

struct ViewState {
    simulator: Simulator,
    viewport_x: i32,       // ビューポート左上の x 座標
    viewport_y: i32,       // ビューポート左上の y 座標
    paused: bool,
    term_cols: u16,        // 端末幅
    term_rows: u16,        // 端末高さ（ステータスバー含む）
}

/// ビューモードのエントリポイント
pub fn run_view_mode(circuit: Circuit) -> Result<(), String> {
    // 1. raw モード + alternate screen に入る
    // 2. Simulator を初期化
    // 3. イベントループ実行
    // 4. 端末をリストア
}
```

### レンダリング手順

1. `terminal::size()` で端末サイズを取得
2. グリッド領域 = `(term_cols, term_rows - 1)` 
3. 各セル `(viewport_x + col, viewport_y + row)` の状態を `SimState::get()` で取得
4. 状態に応じて `#` / `_` / `.` を出力
5. 最下行にステータスバーを描画
6. `stdout().flush()` で一括出力

毎フレーム全画面を再描画する（差分描画は初期実装では不要）。

### エラーハンドリング

- 端末リストアは `Drop` トレイトではなく、関数終了時に明示的に実行する。パニック時のリストアは初期実装では対象外とする
- 回路 JSON のパースエラーはビューモード開始前に報告し、通常終了する

## ステップ

1. `crossterm` を `Cargo.toml` に追加
2. `src/bin/lgcell2/view.rs` を作成し、`ViewState` とレンダリングロジックを実装
3. `main.rs` に `--view` フラグと分岐処理を追加
4. キー入力ハンドリング（q / Space / 矢印）を実装
5. イベントループとシミュレーション自動進行を実装
6. テスト用の回路 JSON で動作確認

## 関連タスク

- [interactive-cli.md](interactive-cli.md): インタラクティブモード（テキストコマンドベース）。`--view` とは排他
- [separate-clap-dependency/README.md](separate-clap-dependency/README.md): feature flag 分離。`crossterm` も `cli` feature にまとめる対象
