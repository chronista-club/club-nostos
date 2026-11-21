# ADR-0012: Lean による Outcome の小さな証明実験

Status: experimental（ローカル spike）

## Context

Lean による形式検証を小さく体験する。最初の対象は、ADR-0002 の
`Outcome` と `map_done`。三つの variant だけで場合分けが完結し、
実行環境や非同期処理をモデル化せずに性質を記述できる。

## Decision

`formal/lean/` に Rust から手で写した独立モデルと証明を置く。
Lean のバージョンを固定し、外部ライブラリなしで実行する。

最初に確かめる性質は次の四つ。

- 恒等変換: `map_done(id, x) = x`
- 合成: `map_done(g, map_done(f, x)) = map_done(g ∘ f, x)`
- `Reborn` の値は変わらない
- `Failed` の値は変わらない

証明を省略する `sorry` や独自の公理を使わず、警告もエラーとして扱う。
誤った恒等性（`+1` しても元と同じ）を証明しようとすると拒否されることも確認する。

## Scope and limitations

対象は Lean モデル上の、純粋で停止する関数による値変換。
Rust ソースを自動翻訳したものではなく、Rust とモデルの対応の証明はない。
Rust の `FnOnce` の副作用、panic、非停止、所有権・Drop の挙動は扱わない。
Rust 実装の保証には、対応関係の検証が別途必要となる。

既存 Rust コード、Cargo 依存、CI の変更は本実験の対象外。
次の候補は `drive_bounded` の中断・再開の合成則だが、本実験には含めない。

## Guide

実行方法と証明の読み方は [Lean 実験ガイド](../../formal/lean/README.md)。
