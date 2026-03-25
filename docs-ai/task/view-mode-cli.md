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
tick:42 | running | (0,0)-(19,4)          (q:quit space:pause arrows:scroll)
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
tick:42 | running | (0,0)-(79,23)          (q:quit space:pause arrows:scroll)
```

- 左寄せで tick 番号、状態、表示領域の座標範囲を表示
  - `(x1,y1)-(x2,y2)`: ビューポートの左上座標と右下座標
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

    if let Some(key) = console.poll_event(timeout)? {
        match key {
            KeyInput::Char('q') => break,
            KeyInput::Char(' ') => paused = !paused,
            KeyInput::Up | KeyInput::Down |
            KeyInput::Left | KeyInput::Right => renderer.scroll(dx, dy),
            _ => {}
        }
    }

    if !paused && elapsed >= TICK_INTERVAL {
        simulator.tick();
        reset_timer();
    }

    let (cols, rows) = console.size()?;
    let frame = renderer.render_frame(simulator.state(), simulator.current_tick(), paused, cols, rows);
    console.write_frame(&frame)?;
}
```

### 依存ライブラリ

- **`crossterm`**: クロスプラットフォーム端末操作（raw モード、キー入力、画面クリア、カーソル制御）
- `Cargo.toml` の `[dependencies]` に `optional = true` で追加し、`cli` feature に含める

### モジュール構成

端末操作（crossterm 依存）とレンダリングロジック（純粋な文字列生成）を分離し、レンダリングロジックを単体テスト可能にする。

```
src/
    platform/
        mod.rs            # platform モジュール
        console.rs        # Console トレイト + CrosstermConsole 実装
    view/
        mod.rs            # view モジュール
        renderer.rs       # ViewRenderer: グリッド・ステータスバーのレンダリング
        renderer_tests.rs # renderer のユニットテスト

src/bin/lgcell2/
    main.rs               # --view フラグ追加、モード分岐
    view.rs               # イベントループ（Console + Simulator + ViewRenderer を結合）
```

#### `Console` トレイト（`src/platform/console.rs`）

端末操作を抽象化するトレイト。crossterm 依存を隔離し、テスト時にスタブへ差し替え可能にする。

```rust
/// 端末入出力を抽象化するトレイト
pub trait Console {
    /// 端末サイズ (cols, rows) を返す
    fn size(&self) -> Result<(u16, u16), String>;
    /// alternate screen + raw モードに入る
    fn enter_alternate_screen(&mut self) -> Result<(), String>;
    /// alternate screen + raw モードから抜ける
    fn leave_alternate_screen(&mut self) -> Result<(), String>;
    /// 画面バッファを書き込む。文字列は (0,0) から端末に直接出力される
    fn write_frame(&mut self, content: &str) -> Result<(), String>;
    /// キーイベントを待つ。timeout=None で無期限待機
    fn poll_event(&self, timeout: Option<Duration>) -> Result<Option<KeyInput>, String>;
}

/// キー入力の抽象表現
pub enum KeyInput {
    Char(char),
    Up,
    Down,
    Left,
    Right,
}
```

`CrosstermConsole`: crossterm を使用した本番実装。`src/platform/console.rs` 内に定義。

`StubConsole`: テスト用スタブ。事前設定したキー入力列を返し、書き込まれたフレームを記録する。テストモジュール内に定義。

#### `ViewRenderer`（`src/view/renderer.rs`）

シミュレーション状態からフレーム文字列を生成する純粋なロジック。端末操作に依存しない。

```rust
pub struct ViewRenderer {
    viewport_x: i32,
    viewport_y: i32,
}

impl ViewRenderer {
    pub fn new(viewport_x: i32, viewport_y: i32) -> Self;

    /// グリッド領域を文字列として生成する
    /// cols x rows の領域を、各セルの状態に応じて '#' / '_' / '.' で埋める
    pub fn render_grid(&self, state: &SimState, cols: u16, rows: u16) -> String;

    /// ステータスバー文字列を生成する
    pub fn render_status_bar(
        &self,
        tick: u64,
        paused: bool,
        cols: u16,
        rows: u16,
        total_width: u16,
    ) -> String;

    /// render_grid + render_status_bar を結合した完全なフレームを生成する
    pub fn render_frame(
        &self,
        state: &SimState,
        tick: u64,
        paused: bool,
        cols: u16,
        rows: u16,
    ) -> String;

    // ビューポート操作
    pub fn scroll(&mut self, dx: i32, dy: i32);
    pub fn viewport(&self) -> (i32, i32);
}
```

#### イベントループ（`src/bin/lgcell2/view.rs`）

`Console` + `Simulator` + `ViewRenderer` を結合するエントリポイント。

```rust
use std::time::{Duration, Instant};

const TICK_INTERVAL: Duration = Duration::from_millis(200);

struct ViewState<C: Console> {
    console: C,
    simulator: Simulator,
    renderer: ViewRenderer,
    paused: bool,
}

/// ビューモードのエントリポイント
pub fn run_view_mode(circuit: Circuit) -> Result<(), String> {
    let console = CrosstermConsole::new();
    run_view_loop(console, circuit)
}

/// Console を注入可能なイベントループ本体
fn run_view_loop<C: Console>(console: C, circuit: Circuit) -> Result<(), String> {
    // 1. console.enter_alternate_screen()
    // 2. Simulator, ViewRenderer を初期化
    // 3. イベントループ実行
    // 4. console.leave_alternate_screen()
}
```

### レンダリング手順

1. `Console::size()` で端末サイズを取得
2. グリッド領域 = `(cols, rows - 1)`
3. `ViewRenderer::render_frame()` でフレーム文字列を生成
   - 各セル `(viewport_x + col, viewport_y + row)` の状態を `SimState::get()` で取得
   - 状態に応じて `#` / `_` / `.` を生成
   - 最下行にステータスバーを追加
4. `Console::write_frame()` で一括出力

毎フレーム全画面を再描画する（差分描画は初期実装では不要）。

### エラーハンドリング

- 端末リストアは `Drop` トレイトではなく、関数終了時に明示的に実行する。パニック時のリストアは初期実装では対象外とする
- 回路 JSON のパースエラーはビューモード開始前に報告し、通常終了する

## テスト方針

`Console` トレイトによる分離と純粋なレンダリング関数により、端末を使わずにユニットテストを行う。

### Unit-Fake test

#### レンダリングロジック（`src/view/renderer_tests.rs`）

`ViewRenderer` の各メソッドは `SimState` を入力として文字列を返す純粋関数であり、端末に依存しない。以下を検証する:

- **`render_grid`**: セルの ON/OFF/不在が `#` / `_` / `.` に正しくマッピングされること
- **`render_grid`（ビューポート）**: ビューポート位置を変えた場合に正しいセルが表示されること
- **`render_grid`（範囲外）**: バウンディングボックス外の領域が `.` で埋められること
- **`render_status_bar`**: tick 番号、running/paused 状態、座標範囲、操作ヘルプが正しくフォーマットされること
- **`render_status_bar`（幅）**: 端末幅に応じてパディングが調整されること
- **`scroll`**: スクロール操作でビューポート座標が正しく更新されること

#### イベントループ（`src/bin/lgcell2/view.rs` 内テスト）

`StubConsole` を使用し、イベントループの振る舞いを検証する。`StubConsole` は事前設定したキー入力列を順に返し、`write_frame` で渡されたフレーム文字列を記録する。

- **終了**: `q` キーでループが終了すること
- **一時停止**: `Space` キーで paused 状態がトグルされること（ステータスバーの表示で確認）
- **スクロール**: 矢印キーでビューポートが更新されたフレームが出力されること
- **自動進行**: paused でない場合、tick が進行したフレームが出力されること
- **alternate screen**: `enter_alternate_screen` / `leave_alternate_screen` が正しい順序で呼ばれること

### Unit-Fake test 以降

Feature-Fake test、Unit-Real test、Feature-Real test は初期実装では作成しない。TUI の実端末テストは手動での動作確認とする。

## ステップ

1. `crossterm` を `Cargo.toml` に追加（`cli` feature に含める）
2. `src/platform/console.rs` を作成し、`Console` トレイトと `CrosstermConsole` を実装
3. `src/view/renderer.rs` を作成し、`ViewRenderer` のレンダリングロジックを実装
4. `src/view/renderer_tests.rs` にレンダリングのユニットテストを追加
5. `src/bin/lgcell2/view.rs` にイベントループを実装（`StubConsole` によるテスト含む）
6. `main.rs` に `--view` フラグと分岐処理を追加
7. テスト用の回路 JSON で手動動作確認

## 関連タスク

- [interactive-cli.md](interactive-cli.md): インタラクティブモード（テキストコマンドベース）。`--view` とは排他
- [separate-clap-dependency/README.md](separate-clap-dependency/README.md): feature flag 分離。`crossterm` も `cli` feature にまとめる対象
