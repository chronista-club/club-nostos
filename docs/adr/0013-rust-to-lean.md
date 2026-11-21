# ADR-0013: 実際の Rust 関数から Lean へつなぐ

Status: experimental

## Context

ADR-0012 は手書き Lean モデルの法則を証明したが、Rust の変更は証明に伝わらない。
次の一歩として、実ソースを機械的に翻訳し、その定義が仕様に一致することを証明する。

## Decision

対象は `crates/nostos-core/src/outcome.rs` の `Outcome` の6メソッド。
`is_done` / `is_reborn` / `is_failed` は該当 variant のときだけ true を返す。
`done` / `reborn` / `failed` は該当 variant の中身をそのまま Some で返し、他は None を返す。
いずれも任意の型・値について、全3分岐を確かめる。
Charon が Rust を読み、Aeneas が Lean 定義を生成する。生成定義は手で修正しない。
証明と期待する性質は別ファイルに置く。ツールの版と再生成手順を固定する。
再生成・検証のスクリプトは既存の技術スタックに合わせて Bun + TypeScript とし、
組み込み API のみを使う。

検証では正しいソースの6つの証明成功に加え、一時コピーの Rust を一メソッドずつ壊し、
再生成後に対応する証明だけが失敗することを確認する。元のソースは変更しない。

## Scope and trust boundary

対象はこの6メソッドの生成モデル上の戻り値。Rust 全体やバイナリ、他の関数は保証しない。
取り出しで破棄される値の `Drop` の副作用・panic・非停止も対象外。
Lean のカーネルに加え、Rust コンパイラと Charon / Aeneas の変換、対象の選び方を
信頼する。抽出対象を opaque な宣言や公理に置き換えて証明を通してはいけない。

既存の Rust の動作・公開 API・Cargo 依存は変更しない。
既存の手書きモデルは入門用として保持する。
既存の Rust テストCIに加え、独立した `Lean proofs` ジョブを実行する。
PRとmainへのpushで実ソースの再生成・6証明・6通りのmutation testを実行し、
生成定義と依存lockに未反映の差分があれば失敗させる。リリースは今回の範囲外。

## Guide

固定バージョン、実行方法、負例の再現と公理依存は
[Rust と Lean をつなぐガイド](../../formal/aeneas/README.md) に記載する。

## Status log

- 2026-11-21: 最初の `is_done` から判定3つ・取り出し3つに拡張。
  各定理の公理を検査し、6通りの独立した Rust mutation で証明の失敗を確認する手順を追加。
- 2026-11-21: ユーザーの依頼により、既存のRustテストを維持したままLean検証をCIに追加。
  Linux配布物の版とSHA-256を固定し、依存ビルドだけをキャッシュする。
