# ADR-0003 — lifecycle ↔ loop dual view

> **Status**: `accepted` (2026-05-16、 review 完了)
> **Date**: 2026-05-16
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0001](0001-bracket-and-outcome.md) Axis C
> **Builds on**: [ADR-0002](0002-outcome-adt.md) (Outcome ADT)

---

## Context

[ADR-0001](0001-bracket-and-outcome.md) Axis C は 「単発 lifecycle」 と 「反復 loop」 を同一 substrate で扱う設計を問うた。 3 案 ── α (別 trait)、 β (iter adapter)、 γ (`Reborn` 自己再帰) ── があり、 暫定傾きは γ。

[ADR-0002](0002-outcome-adt.md) で `Outcome<O, I, E>` が確定し、 `Reborn(I)` が **次回入力を持つ** 形になった。 本 ADR はこれを使って Axis C を実装可能レベルまで decide する。

## Decision

### D1 — 方式は γ (`Reborn` 自己再帰)

loop を `Outcome::Reborn(I)` の自己再帰として表現する。 別 trait (α)・iter adapter (β) は採らない。

`Reborn(I)` は文字通り 「次の生」 の入力を持つ。 loop とは **「`Reborn` が返る限り、 その入力で再び始める」** こと。 nostos の語源 (νόστος = 変容を経た帰還) が、 そのまま loop の制御構造になる。

### D2 — step の抽象は `FnMut(I) -> Outcome<O, I, E>` (本 ADR 範囲では closure)

dual view の substrate は 「`I` を受けて `Outcome<O,I,E>` を返すもの」。 これを **step** と呼ぶ。

- **単発 lifecycle** = step を 1 回呼ぶ = `Outcome` がそのまま結果
- **反復 loop** = step を `Reborn` が続く限り繰り返す

本 ADR の driver (`drive` / `drive_bounded`) は step を **closure `FnMut(I) -> Outcome<O,I,E>`** として受け取る。 `FnMut` は最も許容的な境界で、 `&mut` 状態は capture で扱える。

> **provisional 注記** (review Q3 = 「Axis A まで保留」): step を **名前付き trait** (`Step` 等) として抽象化するか、 Bracket trait を直接 drive できる API を足すかは、 Bracket signature (Axis A、 ADR-0004) と同時に決める。 `drive` が `FnMut` を取ること自体は恒久的に安全 ── Bracket が確定しても `drive(i, |x| bracket.step(x))` で必ず closure に包めるため、 本 ADR の実装は ADR-0004 の結論と非互換にならない。

### D3 — loop driver `drive` (上限なし)

```rust
pub fn drive<O, I, E, F>(initial: I, step: F) -> Result<O, E>
where
    F: FnMut(I) -> Outcome<O, I, E>;
```

- `initial` で 1 回目の step を呼ぶ
- `Done(o)` → loop 終了、 `Ok(o)`
- `Reborn(i)` → `i` で step を再度呼ぶ (継続)
- `Failed(e)` → loop 終了、 `Err(e)`

**戻り値が `Result<O, E>` である理由**: loop が終了する時点で結果は必ず `Done` か `Failed` の二相 ── `Reborn` は 「終了していない」 ことを意味するので終端には現れない。 三相 `Outcome` が loop の終端で二相に収束する。 これは ADR-0002 D4 (「`Outcome` 単体を `Result` に畳まない」) と矛盾しない ── 畳んでいるのは Outcome 単体ではなく **loop の意味論的終端**。

### D4 — 上限つき driver `drive_bounded`

`Reborn` が永遠に続くと無限 loop になる。 上限つきの変種を用意する:

```rust
pub fn drive_bounded<O, I, E, F>(initial: I, max_steps: usize, step: F) -> Outcome<O, I, E>
where
    F: FnMut(I) -> Outcome<O, I, E>;
```

- `max_steps` 回以内に `Done` / `Failed` → その `Outcome` を返す
- `max_steps` 回で打ち切られたら、 **最後の `Reborn(i)` をそのまま返す** (review Q2 で確定)

戻り値が `Outcome<O,I,E>` (三相のまま) なのが要点 ── `Reborn(i)` は 「上限で中断、 ここから再開可能」 を意味し、 呼び出し側は同じ `i` で `drive` / `drive_bounded` を呼べば続行できる。 三相 ADT が中断・再開の表現として完全に機能し、 専用の `Interrupted` 型を増やさずに済む。

`drive` (上限なし) と `drive_bounded` (上限あり) の使い分け:

| | 終端 | 戻り値 | 無限 loop |
|--|--|--|--|
| `drive` | step が `Done`/`Failed` を返す責任 | `Result<O, E>` | step 次第 |
| `drive_bounded` | `max_steps` で保証 | `Outcome<O, I, E>` (`Reborn` = 中断) | しない |

### D5 — 単発 lifecycle に専用 API は要らない

単発 lifecycle = step を 1 回呼ぶ = `step(initial)` で `Outcome` が得られる。 driver は不要。

つまり dual view は ── **同じ step 関数を、 直接呼べば lifecycle、 `drive` に渡せば loop**。 「lifecycle 側」 は `Outcome` そのもの (ADR-0002 で完結)、 「loop 側」 だけが本 ADR の `drive` / `drive_bounded`。 dual view のために新しい lifecycle 型は導入しない。

### D6 — ファイル構成

```
crates/nostos-core/src/
├── lib.rs        # pub use drive::{drive, drive_bounded}
├── outcome.rs    # ADR-0002
└── drive.rs      # drive / drive_bounded + tests
```

module 名は `drive` ── `loop` は Rust 予約語、 `cycle` は ADR-0002 の `Cycle` alias と紛らわしいため避ける。 driver 名は `drive` / `drive_bounded` (review Q1 で確定)。

## Consequences

### Positive

- loop が言語機能ではなく **`Outcome` の値** で表現される ── step を data として渡せ、 graph editor (Axis D) で node 化する素地になる
- `drive_bounded` の戻り値が `Outcome` なので、 中断・再開が型で表現される (cooperative scheduling / step 実行に直結)
- Bracket trait (Axis A) を待たずに実装・テストできる

### Negative

- `drive` は無限 loop を防げない ── step の設計責任。 `drive_bounded` を併設して緩和
- step 抽象が provisional (closure のみ) ── Bracket trait 確定後に 「名前付き `Step` trait / Bracket 直接 drive」 を足すか再検討する余地が残る (ADR-0004)

### Neutral

- `drive` 系は `Cycle<T, E>` (= `Outcome<T, T, E>`) と相性が良い ── 成果と次回入力が同型なら step は `FnMut(T) -> Outcome<T, T, E>`

## Resolved Questions

draft 段階の Open Questions は review (2026-05-16) で以下に確定:

1. **driver 名** → `drive` / `drive_bounded` (D3 / D6)
2. **`drive_bounded` の上限到達表現** → 最後の `Reborn(i)` をそのまま返す。 専用 `Interrupted` 型は作らない (D4)
3. **step 抽象** → 本 ADR では closure `FnMut` で実装。 名前付き `Step` trait / Bracket 直接 drive の要否は **ADR-0004 (Axis A) に保留** (D2 provisional 注記)

---

> 本 ADR は `accepted`。 次は ADR-0003 に基づくテストリスト作成 (t-wada 流) → 実装。
