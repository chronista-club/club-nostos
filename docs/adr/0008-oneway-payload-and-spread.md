# ADR-0008 — OneWay の payload 獲得と Spread routing

> **Status**: `accepted` (2026-05-19、 review 完了)
> **Date**: 2026-05-19
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0006](0006-voyage.md) OQ2 を decide / D3 を部分 supersede ・ [ADR-0005](0005-graph-substrate.md) D5 を具体化 ・ [ADR-0007](0007-graph-design.md)
> **Trigger**: nostos-graph 実装 (`mem_1CbBXLAUZkT8gy8mBBC7FW`) が炙り出した設計の隙間

---

## Context

nostos-graph (ADR-0007) の実装で **設計の隙間**が露出した:

- [ADR-0005](0005-graph-substrate.md) D5 は 「`OneWay(Spread::Ok)` = fan-out edge (空間の再帰)」 と書いた
- だが [ADR-0006](0006-voyage.md) D3 は 「`Voyage::OneWay` は core で **bare** (payload 無し)」 と確定
- [`Node::drive`](0007-graph-design.md) は `V` を返す/受ける。 bare な `OneWay` を fan-out しようにも、 **近傍 node を駆動する値が無い**

実装は RoundTrip routing を完全実装し、 `OneWay` は暫定的に node の sink として扱った。 `Spread` 型は hollow になるため実装を見送った。 本 ADR がこの隙間を埋める。

### 隙間の正体 — ADR-0006 D3 の conflation

ADR-0006 D3 は 「`OneWay` bare」 の根拠を 「拡散 (`Spread`) は topology の概念で core の領分でない」 とした。 しかし **2 つの別概念を conflate している**:

- **payload** ── 「一方向に *何を* 発したか」。 notification には content がある。 これは **単一 voyage の事実**であり、 topology を前提にしない
- **spread** ── 「発したものが graph 構造を *さらに伝播* するか」。 これは **topology の概念**

ADR-0006 D3 が正しいのは後者 (spread は graph の責務) だけ。 前者 (payload) を巻き込んで bare にしたのが隙間の源。 payload と spread を分離すれば解ける。

## Decision

### D1 — `OneWay` は payload を持つ

`Voyage::OneWay` に payload を持たせる。 [ADR-0006](0006-voyage.md) OQ2 (「`OneWay` の将来 payload」) を **decide**、 D3 の 「bare」 部分を **supersede** する。

```rust
pub enum Voyage<O, I, E> {
    /// 往相 ── O を一方向に発した。 還を持たない。
    OneWay(O),
    /// 還相 ── 往って還った。
    RoundTrip(Outcome<O, I, E>),
}
```

理由: 一方向 voyage も 「発した content」 を持つ ── notification は中身を運ぶ。 「還が無い」 ことと 「payload が無い」 ことは別。 ADR-0006 D3 の spread-is-graph-責務 は維持し、 payload だけ core に戻す。

### D2 — payload 型は `O` を reuse (`Voyage<O,I,E>` 3-param 維持)

`OneWay` の payload は Done と同じ型パラメータ `O` を使う。 `Voyage` は 3-param のまま。

`O` は 「voyage の主たる payload 型」 ── `Done(O)` は 「`O` を持って還った」、 `OneWay(O)` は 「`O` を一方向に発した」。 両者は *還ったか否か* で分かれ、 運ぶものは同じ `O`。 graph では `Voyage<V,V,V>` → `OneWay(V)` で routing に乗る。

(4 つ目の型パラメータ `W` で `OneWay(W)` とする案は Open Questions。)

### D3 — `Spread` は nostos-graph 層の routing policy

`Spread` (拡散 policy) は ADR-0006 D3 の通り **`nostos-graph` の責務**に留める。 core は payload を持つ `OneWay` を提供するだけ ── それが graph をさらに伝播するかは graph の topology 判断。

```rust
// nostos-graph 側
pub enum Spread {
    /// fan-out ── payload を全近傍 node へ broadcast
    Ok,
    /// sink ── その node で消費、 伝播しない
    Ng,
}
```

payload (core) と spread (graph) の分離が本 ADR の核。

### D4 — nostos-graph 評価器の `OneWay` handling

`Node::drive` が `OneWay(v)` を返した時、 評価器は node の `Spread` policy を引く:

| Spread | 振る舞い | 再帰軸 |
|--------|---------|--------|
| `Spread::Ok` | `v` を全近傍 node へ fan-out (各近傍を `v` で駆動) | **空間** |
| `Spread::Ng` | sink ── `v` は消費され伝播しない | ── |

これで [ADR-0005](0005-graph-substrate.md) D5 の 「空間の再帰」 が実装可能になる ── `Reborn` の self-loop (時間) / `OneWay(Spread::Ok)` の fan-out (空間) / `Graph: Node` の入れ子 (深さ) の **再帰三軸**が揃う。

routing table は node ごとに `Spread` policy と fan-out 先 (近傍 node の集合) を持つ。

### D5 — 既存実装への影響

- `nostos-core` `voyage.rs` ── `OneWay` → `OneWay(O)`。 `Voyage` は当日 (2026-05-19) 実装、 pre-1.0、 consumer ゼロのため breaking change のコストは最小
- `nostos-graph` ── 評価器に `OneWay` の Spread handling を追加、 `Spread` 型を実装、 routing table に近傍集合を追加
- `Bracket` は不変 ── RoundTrip 専用 (ADR-0006 D4 維持)

## Consequences

### Positive

- 実装が炙り出した隙間が塞がる ── `OneWay` fan-out が値を運べる
- 「再帰三軸」 (時間 / 空間 / 深さ) が初めて全軸 実装可能に
- payload と spread の分離で、 ADR-0006 D3 の正しい部分 (spread = graph 責務) は保たれる

### Negative

- `Voyage::OneWay` の breaking change ── ただし実装当日・consumer ゼロで影響最小
- `O` の reuse は 「OneWay payload と Done payload が同型」 を強いる ── 多くの用途で自然だが、 別型にしたい consumer は変換を要する (Open Questions)

### Neutral

- ADR-0006 D3 は 「payload」 部分のみ supersede ── 「spread は graph 責務」 の判断は本 ADR D3 が引き継ぐ

## Resolved Questions

review (2026-05-19) で確定:

1. **payload 型** → `O` を reuse、 `Voyage<O,I,E>` 3-param 維持 (D2)
2. **`Spread` の variant** → `{ Ok, Ng }` 2 値で確定 (D3)。 選択的 spread が要れば後続 ADR で variant 追加
3. **fan-out の評価順序** → **未規定** (consumer 非依存)。 graph の結果は fan-out 順序に依存しない設計を推奨し、 評価器の処理順は spec しない

---

> **昇格** (2026-05-19): review で payload 型 (`O` reuse) / `Spread` variant (`{ Ok, Ng }`) / fan-out 順序 (未規定) を確定。 本 ADR を `accepted` に昇格。 次は `voyage.rs` の `OneWay(O)` 化 + nostos-graph の `Spread` / fan-out 実装。
