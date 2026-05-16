# ADR-0002 — Outcome ADT の設計

> **Status**: `accepted` (2026-05-16、 review 完了)
> **Date**: 2026-05-16
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0001](0001-bracket-and-outcome.md) Axis B
> **Related**: [[nostos-founding-decision]] (creo-memories `mem_1Cb35YiGHG1f7UdXyyt16L`)

---

## Context

[ADR-0001](0001-bracket-and-outcome.md) は Outcome ADT を **最小単位** と位置づけ、 深掘り順序の起点とした。 本 ADR は ADR-0001 Axis B を **実装可能なレベルまで decide** する。

Outcome ADT は nostos の **意味論的 core** ── 帰還 (νόστος) の三相を Rust の型として表現する。 ここが固まると Axis C (lifecycle ↔ loop dual) と Axis A (Bracket signature) の関連型が連動して決まる。

## Decision

### D1 — variant 集合は 3 つで固定

```rust
pub enum Outcome<O, I, E> {
    Done(O),
    Reborn(I),
    Failed(E),
}
```

| variant | 意味 | 値 |
|---------|------|----|
| `Done(O)` | 旅が完了し、 成果を持ち帰った | 成果 `O` |
| `Reborn(I)` | 変容を経て次の生へ ── 帰還の core meaning | 次回入力 `I` |
| `Failed(E)` | 失敗 | エラー `E` |

`Pending` / `Cancelled` / `Suspended` 等は **core に入れない**。 nostos の Outcome は 「帰還が完了した三相」 を表す型であり、 「まだ帰っていない」 状態は別の関心事 (Bracket の `Active` 側、 もしくは別 ADT)。 core を 3 variant に保つ。

### D2 — generic parameter は 3 つ `<O, I, E>` + `Cycle` alias

`Done` / `Reborn` / `Failed` が各々独立の型を持つ。 多くの loop では `O` と `I` が同型になる (成果がそのまま次の入力) ため、 その頻出形に type alias を与える:

```rust
/// 成果と次回入力が同型の Outcome (= loop で頻出)。
pub type Cycle<T, E> = Outcome<T, T, E>;
```

`Cycle` は本 ADR の実装に含める (review Q3 で確定)。

### D3 — variant naming は `Done` / `Reborn` / `Failed`

3 variant をすべて過去分詞で揃える ── Outcome は 「帰還が完了した状態」 を表す状態 enum であり、 品詞を統一するのが意味的に正確 (review Q1 で確定)。

- `Reborn` は **fix**。 ADR-0001 で 「nostos の意味論的 core」 と位置づけた語。 「変容を経た帰還」 = 別人として再び生まれる、 が `Reborn` に宿る。
- `Failed` は当初案 `Fail` (簡潔さ重視) から変更。 `Done` / `Reborn` との grammatical 統一を優先した。

method 名もこれに連動する (`is_failed()` / `failed()` / `map_failed()`)。

### D4 — `Result` 相互運用は一方向のみ

`Result<O, E>` → `Outcome` を **第一級**で提供 (`From` 実装):

```rust
impl<O, I, E> From<Result<O, E>> for Outcome<O, I, E> {
    // Ok(o)  -> Done(o)
    // Err(e) -> Failed(e)
}
```

`Result` には `Reborn` に相当する variant が無いため、 この変換は `Reborn` を**生成しない** (情報損失なし、 自然な埋め込み)。

**逆方向 (`Outcome` → `Result`) は提供しない** (review Q2 = 案b で確定)。 `Reborn` を `Result` の二相に畳むと 「Outcome は `Result` の亜種」 という誤解を招く。 nostos の Outcome は **三相の独立した代数** であり、 値の取り出しは D5 の `done()` / `reborn()` / `failed()` (各々 `Option`) で行う。

### D5 — core methods (minimal set)

`Result` / `Option` の API に倣い、 以下を最小セットとする:

| 種別 | method | 戻り値 |
|------|--------|--------|
| 述語 | `is_done()` / `is_reborn()` / `is_failed()` | `bool` |
| 取り出し | `done()` / `reborn()` / `failed()` | `Option<O>` / `Option<I>` / `Option<E>` |
| 変換 | `map_done(f)` / `map_reborn(f)` / `map_failed(f)` | `Outcome<..>` (該当 variant のみ変換) |

`into_result()` および `unwrap` 系は **本 ADR では入れない**。 前者は D4 の通り。 後者は 3 variant あり 「成功/失敗」 が `Result` ほど自明でないため、 panic API は需要が見えてから足す。

### D6 — derive

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
```

各 generic parameter が該当 trait を満たすときのみ有効 (derive の標準挙動)。 `PartialOrd` / `Ord` は **derive しない** ── `Done` / `Reborn` / `Failed` に全順序を与える意味論が無い。

### D7 — `no_std` 対応

`nostos-core` を **`#![no_std]`** とする。 Outcome ADT は enum + method のみで `std` にも `alloc` にも依存しない。 no_std にしておくことで embedded / wasm でも使える。

> Bracket (Axis A) が将来 `std` を要する場合は、 その時点で `std` feature flag を導入する。 Outcome 段階では no_std-native。

### D8 — MSRV は `1.95` で fix

Outcome ADT は GAT を使わない単純な enum。 ADR-0001 Open Question 2 (MSRV) について、 **Outcome に関する限り `1.95` で確定**。 GAT 安定化への追従は Axis A (Bracket) で別途判断する。

### D9 — ファイル構成

```
crates/nostos-core/src/
├── lib.rs        # #![no_std] + pub mod outcome + pub use outcome::{Outcome, Cycle}
└── outcome.rs    # Outcome<O,I,E> + Cycle alias + impl + tests
```

`lib.rs` で `pub use outcome::{Outcome, Cycle};` し、 consumer は `use nostos::Outcome;` で参照する。

## Consequences

### Positive

- nostos の意味論的 core (`Reborn` = 次の生) が、 Rust の型として最小コストで表現される
- `into_result()` を持たないことで 「Outcome は三相の独立代数」 という設計意図が API に表れる
- `no_std` により適用範囲が広い (embedded / wasm)
- `Result` からの自然な埋め込みで、 既存 Rust コードからの移行 path がある

### Negative

- generic parameter 3 つは型注釈がやや冗長 (`Outcome<i32, i32, String>`) ── `Cycle<T, E>` alias で頻出形は緩和される
- `Outcome` → `Result` 変換が無いため、 `Result` ベースの API へ橋渡しする利用者は `done()` 等で明示的に分岐する必要がある (= 意図的な設計)

### Neutral

- Axis C (lifecycle ↔ loop dual) は `Reborn(I)` を使った自己再帰として表現する見込み (ADR-0001 暫定傾き) ── 本 ADR はその土台を据えるのみで、 dual view 自体は ADR-0003 で扱う

## Resolved Questions

draft 段階の Open Questions は review (2026-05-16) で以下に確定:

1. **variant naming** → `Done` / `Reborn` / `Failed` (過去分詞で統一、 D3)
2. **`into_result()` の `Reborn` 畳み込み** → 案b: `into_result()` を提供しない (D4 / D5)
3. **type alias `Cycle`** → 本実装に含める (D2)

---

> 本 ADR は `accepted`。 次は ADR-0002 に基づくテストリスト作成 (t-wada 流) → 実装。
