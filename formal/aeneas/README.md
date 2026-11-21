# 実際の Rust と Lean をつなぐ

`Outcome` の判定3メソッドと取り出し3メソッドを Rust から自動変換し、
それぞれの戻り値が仕様に一致することを証明する。
判断の背景は [ADR-0013](../../docs/adr/0013-rust-to-lean.md)。

## まず実行する

導入済みの環境では、リポジトリのルートで次を実行する。

```bash
bun formal/aeneas/check.ts
```

毎回、実際の Cargo crate から対象の6メソッドと必要な型を Charon で抽出し、
Aeneas で [Nostos.lean](Nostos.lean) を再生成する。
その生成定義を読み込んで [NostosProof.lean](NostosProof.lean) の証明を検査する。
成功時は `PASS: current Rust matches all 6 specifications` に続いて対象名が表示される。

```text
crates/nostos-core/src/outcome.rs   ← 実際の Rust
    ↓ Charon（対象の6メソッドを個別指定）
.lake/nostos.llbc                 ← 中間表現
    ↓ Aeneas
Nostos.lean                       ← 自動生成。手で修正しない
    ↓ import
NostosProof.lean                  ← 期待する仕様と、一致することの証明
```

`lake build` 単独でも保存済みの Lean 定義の証明は確認できるが、Rust の再抽出は行わない。
**Rust を変更したときは `check.ts` を使う。** 再生成された `Nostos.lean` の差分もレビューする。

## 何を証明したのか

以下の6つについて、任意の型 O / I / E と任意の値を扱う。
各仕様は Done / Reborn / Failed の全分岐を含む。

| Rust メソッド | 仕様 |
|---|---|
| `is_done` | Done のときだけ true |
| `is_reborn` | Reborn のときだけ true |
| `is_failed` | Failed のときだけ true |
| `done` | Done の中身をそのまま Some で返し、他は None |
| `reborn` | Reborn の中身をそのまま Some で返し、他は None |
| `failed` | Failed の中身をそのまま Some で返し、他は None |

生成された `Outcome.is_done` の戻り値は `Result Bool`。
Rust の成功・失敗・非停止を表すための Aeneas 側の型であり、Rust の `Result` 型とは別物。
`Result.ok true` は「正常終了して true を返す」を表す。

証明する命題は次の形になっている。

```lean
theorem is_done_matches_spec {O I E : Type} (x : Outcome O I E) :
    Outcome.is_done x = Aeneas.Std.Result.ok (expectedIsDone x) := by
  cases x <;> rfl
```

`Outcome.is_done` は Rust から生成した関数。
`expectedIsDone` は手書きの仕様で、Done に true、Reborn / Failed に false を割り当てる。
任意の型 O / I / E と任意の値 x について、生成関数は正常終了して仕様どおりの結果を返す。
取り出し3メソッドの戻り値は `Result (Option O)` などで、
`Result.ok (some o)` と中身を含めて比較する。`Drop` の副作用や panic はこのモデルの保証外。

## Rust 側を壊すと証明も落ちる

```bash
bun formal/aeneas/check.ts --mutation-test
```

まず現在の実装の6つの証明を確認する。その後、一時コピーの Rust だけを、
**毎回元のソースから一メソッドずつ**壊す。

| 対象 | 一時コピーに入れる誤り | 証明が失敗する分岐 |
|---|---|---|
| `is_done` | Done の代わりに Reborn を判定 | Done / Reborn |
| `is_reborn` | Reborn の代わりに Failed を判定 | Reborn / Failed |
| `is_failed` | Failed の代わりに Done を判定 | Done / Failed |
| `done` | Some(o) の代わりに None を返す | Done |
| `reborn` | Some(i) の代わりに None を返す | Reborn |
| `failed` | Some(e) の代わりに None を返す | Failed |

各コピーから別のディレクトリに Lean を生成し、**同じ仕様・同じ証明**を検査する。
対応する分岐で `rfl` が失敗し、Lean が終了コード 1 を返すことを確認する。
対象の定理だけにエラー回復用の `sorryAx` が現れ、他の5つの公理検査は通る必要がある。
ツールの起動失敗やコンパイル失敗を「期待した失敗」として扱わない。

スクリプト全体は、6つ全ての期待どおりの拒否を確認できた場合に終了コード 0 になる。
`PASS: all 6 intentionally broken copies were rejected; original files are unchanged` が成功の印。
元の Rust と保存済み生成定義は変えず、一時コピーは削除する。
画面には一時コピーの対象名と期待した拒否の結果を表示し、
Lean のエラー詳細は `.lake/mutation-<メソッド名>.log` に残す。
このログのエラーや `sorryAx` は、意図的に壊したコピーのもので、現在の実装のエラーではない。

## 新しい Mac での準備

以下は Apple Silicon macOS 用。`curl` / `tar` / `shasum` / Bun（1.4.5 で動作確認）、
`rustup`、Lean のバージョン管理ツール `elan` が必要。
スクリプトは Bun と組み込み API だけで動き、npm パッケージの追加は不要。
elan がなければ `brew install --formula elan-init`、Bun がなければ `brew install oven-sh/bun/bun` で導入する。

リポジトリのルートから実行する。括弧内でのみ `set -eu` を有効にする。

```bash
(
  set -eu
  tool_dir="${XDG_DATA_HOME:-$HOME/.local/share}/nostos-formal/aeneas-2e3d448"
  mkdir -p "$tool_dir"
  curl -fL \
    https://github.com/AeneasVerif/aeneas/releases/download/nightly-2026.11.21-2e3d448/aeneas-macos-aarch64.tar.gz \
    -o "$tool_dir/aeneas-macos-aarch64.tar.gz"
  printf '%s  %s\n' \
    5b35676c6c77f70a12f0398a30070368bdf42b2d36636d053e611166ccfc1a9e \
    "$tool_dir/aeneas-macos-aarch64.tar.gz" | shasum -a 256 -c -
  tar -xzf "$tool_dir/aeneas-macos-aarch64.tar.gz" -C "$tool_dir"
  rustup toolchain install nightly-2026-09-17 --profile minimal \
    --component rustc-dev,rust-src,llvm-tools-preview
  cd formal/aeneas
  lake update
  bun check.ts --mutation-test
)
```

配布物には対応する Charon も含まれる。既存の Rust のデフォルト toolchain は変更しない。
Lean / Mathlib / Aeneas の初回ダウンロード・キャッシュ展開には時間と数 GB のディスクを使う。
`.lake/` は Git の対象外。依存の版は `lake-manifest.json` に記録する。
ツールを別の場所に置く場合は `AENEAS_HOME` でそのディレクトリを指定する。

| 要素 | 固定した版 |
|---|---|
| Aeneas | `2e3d4488280bb8a95411db81019a7854a784861f` |
| Charon | `9e193f0317dacc822cc864c1798b117c70dcf7df`（0.1.322） |
| 抽出用 Rust | `nightly-2026-09-17` |
| Lean | `4.36.0` |

## 保証の範囲と、信頼しているもの

- 保証対象は上記6メソッドの生成モデル上の戻り値。nostos 全体や他の関数の証明ではない。
  `map_done` / `map_reborn` / `map_failed` と `From<Result>` はまだ対象外。
  取り出しで破棄する値の `Drop` の副作用・panic・非停止も保証しない。
- Rust コンパイラと Charon / Aeneas が正しく変換すること、Lean のカーネルを信頼する。
  配布バイナリや抽出対象の選択も信頼の境界に含まれる。
- 今回の抽出は通常の Cargo crate に対し、`--start-from` で対象を指定する。
  手で切り出した代用品や、関数本体を隠す `opaque` の指定は使わない。
- `--sysroot default` を使用する。対象の6メソッドは外部関数の手書きモデルを必要としない。
- 抽出時のコンパイラは Charon 指定の nightly。プロジェクト指定の Rust 1.95 や
  配布バイナリの機械語まで同じ振る舞いであることの証明ではない。
- 証明は Lean 標準の `propext` と `Quot.sound` に依存する。
  前者は命題の外延性、後者は商型の基本原理。Aeneas の結果モデルを通じて使われる。
  `check.ts` は6つ全ての定理についてそれ以外の公理依存や公理レポートの欠落を拒否し、
  Lean の警告もエラーとして扱う。
- Rust テストも引き続き必要。Rust 実装の変更・バージョン更新は行っていない。

## CI

[GitHub Actions](../../.github/workflows/ci.yml) の `Lean proofs` ジョブで、
main 向けの全PRとmainへのpushごとに `bun formal/aeneas/check.ts --mutation-test` を実行する。
既存の Rust テスト（Linux / macOS）、fmt、Clippy も継続する。

- Ubuntu 24.04上で、固定したAeneas / Charon、Rust nightly、Lean、Bunを使用する。
  Aeneasとelanの配布アーカイブはSHA-256を確認してから展開する。
- Lakeの依存lockに従って依存を取得し、Mathlibのビルド済みキャッシュを利用する。
  `.lake/packages` をOS・アーキテクチャ・依存lockなどに対応するキーでキャッシュする。
  Rustからの抽出と6つの証明の検査は毎回実行する。
- 再生成した `Nostos.lean` と依存lockにコミットとの差分があればCIは失敗する。
  Rust変更に伴う生成定義の更新漏れも検出する。
- 負例の検証ログは `lean-mutation-logs` artifactとして7日間保存する。

Leanは対象6メソッドの生成モデルについて仕様との一致を確認し、Rustテストは
実際のRust実行や他の機能を確認する。両方を検証する構成とする。

公式資料: [Aeneas](https://github.com/AeneasVerif/aeneas/tree/2e3d4488280bb8a95411db81019a7854a784861f)、
[Charon](https://github.com/AeneasVerif/charon/tree/9e193f0317dacc822cc864c1798b117c70dcf7df)。
