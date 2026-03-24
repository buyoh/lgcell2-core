# lgcell2 --interactive: インタラクティブモード

`lgcell2` に `--interactive` フラグを追加し、標準入力から改行ベースのコマンドを受け付けて回路の編集・シミュレーションをインタラクティブに操作できるモードを実装する。

作成日: 2026-03-24
ステータス: 設計完了（未実装）

## 背景・動機

既存の `lgcell2` CLI は「JSON 入力 → 一括シミュレーション → 結果出力」のバッチ処理専用であり、以下ができない:

- 回路構造の段階的な編集（ワイヤの設置・撤去）
- シミュレーションの段階的な実行（任意の tick 数ずつ進める）
- 途中経過の確認（特定セルの状態取得）

`lgcell2 --interactive` はこれらを改行ベースのテキストコマンドで提供する。人間が直接対話することを想定し、レスポンスは読みやすいテキストで返す。

## 設計・方針

### コマンドプロトコル

1 行 = 1 コマンド。レスポンスは人間が読みやすいテキストで返す。

#### コマンド一覧

| コマンド | 構文 | 説明 |
|---------|------|------|
| wire add | `wire add <sx> <sy> <dx> <dy> <positive\|negative>` | ワイヤを追加する。セルは自動生成される |
| wire remove | `wire remove <sx> <sy> <dx> <dy> <positive\|negative>` | 一致するワイヤを 1 本削除する |
| compile | `compile` | 現在のワイヤ一覧から Circuit を構築し Simulator を初期化する |
| reset | `reset` | Simulator の状態を初期化する（回路構造は維持） |
| tick | `tick <n>` | n tick 進める |
| get | `get <x1>,<y1> [<x2>,<y2> ...]` | 指定セルの現在値を取得する |

#### レスポンス形式

成功:
```
OK
OK compiled: 3 cells, 2 wires
OK tick: 10
OK (0,0)=0 (1,0)=1
```

エラー:
```
ERR not compiled yet
ERR unknown command: foo
```

#### コマンド別レスポンス詳細

- **wire add / wire remove**: `OK`
- **compile**: `OK compiled: <セル数> cells, <ワイヤ数> wires`
- **reset**: `OK`
- **tick**: `OK tick: <現在の tick 番号>`
- **get**: `OK (x,y)=<0|1> (x,y)=<0|1> ...`（指定順でスペース区切り）

### 状態管理

セッションは以下の 2 状態を遷移する:

```
[Editing] --compile--> [Compiled]
[Compiled] --wire add/remove--> [Editing]
[Compiled] --reset--> [Compiled] (状態のみ初期化)
```

- **Editing 状態**: ワイヤの追加・削除と `compile` が可能。`tick` / `get` / `reset` はエラーを返す
- **Compiled 状態**: 全コマンドが可能。ワイヤの追加・削除を行うと Editing 状態に戻る

### CLI 引数

既存の `lgcell2` CLI に `--interactive` (`-i`) フラグを追加する。

```
lgcell2 --interactive [file]
lgcell2 -i [file]
```

- `--interactive` / `-i`: インタラクティブモードで起動する
- `file`: 回路定義 JSON ファイル（省略可）。指定時はワイヤ一覧として Editing 状態に投入される（自動 compile はしない）。省略時は空の状態で開始
- `--ticks` は `--interactive` と排他。両方指定時はエラーを返す

`--interactive` 指定時、file 引数があってもなくても標準入力はコマンド入力に使用される（JSON 読み込みには使われない）。
`--interactive` を指定しない場合の動作は従来通り。

### モジュール構成

インタラクティブモードのロジックは `src/interactive/` モジュールとしてライブラリ側に配置する。CLI 固有の stdin/stdout ループのみ `src/bin/lgcell2/` に追加する。

```
src/
    interactive/
        mod.rs        # pub use
        command.rs    # Command enum とパーサー
        session.rs    # InteractiveSession: 状態管理とコマンド実行
src/bin/lgcell2/
    main.rs           # --interactive フラグ追加、分岐処理
```

#### command.rs

```rust
pub enum Command {
    WireAdd { src: Pos, dst: Pos, kind: WireKind },
    WireRemove { src: Pos, dst: Pos, kind: WireKind },
    Compile,
    Reset,
    Tick(u64),
    Get(Vec<Pos>),
}

pub fn parse_command(line: &str) -> Result<Command, String> { ... }
```

#### session.rs

```rust
/// インタラクティブセッションの状態を管理する。
pub struct InteractiveSession {
    wires: Vec<Wire>,
    simulator: Option<Simulator>,
}
```

- `wires`: 編集可能なワイヤ一覧。`wire add` / `wire remove` で直接操作する
- `simulator`: `compile` 後に生成。`Some` のとき Compiled 状態
- `execute(&mut self, cmd: Command) -> String` でコマンドを処理し、レスポンス文字列を返す
- `Wire` 追加時にセルは自動推論される（`Circuit::new` に委譲）

#### main.rs の変更

```rust
#[derive(Debug, Parser)]
#[command(name = "lgcell2")]
struct Cli {
    /// 回路定義 JSON ファイル。省略時は標準入力から読み込み。
    file: Option<PathBuf>,

    /// シミュレーションする tick 数
    #[arg(short, long, default_value_t = 100)]
    ticks: u64,

    /// インタラクティブモード
    #[arg(short, long)]
    interactive: bool,
}
```

`interactive` が true の場合、`run_interactive()` に分岐する。`BufRead::lines()` で stdin を 1 行ずつ読み、`parse_command` → `session.execute` → stdout に出力するループ。

### セルの自動管理

- **ワイヤ追加時**: セルの管理は `compile` 時に行う（`wires` の端点から自動生成）
- **ワイヤ撤去後**: 次回の `compile` で不要なセルは自然に除外される（ワイヤ端点に存在しないセルは生成されない）

### テスト方針

`command.rs` と `session.rs` にそれぞれ Unit テストを作成する。

- **command_tests**: 各コマンドのパース成功・失敗ケース
- **session_tests**: コマンド実行による状態遷移、エラーケース（未コンパイル時の tick 等）、wire add/remove の正常動作、compile → tick → get のフロー

`InteractiveSession` は stdin/stdout に直接依存しない（コマンドを受け取りレスポンスを返す純粋な構造体）ため、モック不要でテスト可能。

### 既存タスクとの関係

- **separate-clap-dependency**: 既存の `lgcell2` バイナリへの統合のため、追加の `[[bin]]` セクションは不要。feature flag 対応にも影響しない

## ステップ

1. `src/interactive/command.rs` — Command enum とパーサーの実装 + テスト
2. `src/interactive/session.rs` — InteractiveSession の実装 + テスト
3. `src/interactive/mod.rs` — モジュール公開
4. `src/lib.rs` — `pub mod interactive;` 追加
5. `src/bin/lgcell2/main.rs` — `--interactive` フラグ追加、分岐処理の実装
6. 結合テスト（手動動作確認）
