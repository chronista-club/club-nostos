# ADR-0007 — nostos-graph 具体設計 (Node / Graph / Value / 評価)

> **Status**: `accepted` (2026-05-18、 review 完了)
> **Date**: 2026-05-18
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0005](0005-graph-substrate.md) (Axis D framing) の OQ1-3 を decide
> **Builds on**: [ADR-0004](0004-bracket-and-driver.md) (Bracket) / [ADR-0006](0006-voyage.md) (Voyage)

---

## Context

[ADR-0005](0005-graph-substrate.md) は Axis D を framing し、 `Node` / `Graph` の具体 signature・graph value 型・評価戦略を後続に委ねた (OQ1-3)。 本 ADR がそれを decide し、 `crates/nostos-graph/` 実装の土台とする。

ADR-0005 の確定事項 (D1-D6) ── node=Bracket / edge=Voyage routing / type-erased `Node` / 別 crate / front-end は外 / `Graph: Node` 自己入れ子 ── は前提として動かさない。

## Decision

### D1 — graph value 型: `Graph<V>` を generic 化 (homogeneous)

graph 上を流れる値の型を `Graph` / `Node` の **generic parameter `V`** とする。 1 つの graph は value 型 `V` で homogeneous。

ADR-0005 OQ1 は `Box<dyn Any>` vs 専用 `Value` enum の二択を挙げたが、 **第三の道**を採る:

- `Box<dyn Any>` ── downcast の実行時 panic 余地、 `Any` は安定だが downcast に alloc
- 専用 `Value` enum ── 流せる型が閉じる
- **generic `<V>`** (採用) ── consumer が `V` を選ぶ。 downcast 不要・型安全・no_std 寄りを保つ。 graph 内は単一型

cost: 1 つの graph が port ごとに異なる型を混ぜられない。 heterogeneous-port は後続 (Open Questions)。

### D2 — `Node<V>` trait

```rust
pub trait Node<V> {
    /// 入力を受け、 Voyage を返す。 enter + exit を 1 手に畳む。
    fn drive(&self, input: V) -> Voyage<V, V, V>;
}
```

- `drive` は [`Bracket`](0004-bracket-and-driver.md) の `enter` + `exit` を **1 メソッドに畳む** ── graph から Active 相は node 内部の不透明事
- 戻り値は [`Voyage`](0006-voyage.md) ── node は `RoundTrip(Outcome)` も `OneWay` も emit できる (ADR-0005 OQ6 = OneWay-emitting node は `drive` が `Voyage::OneWay` を返す node、 と解決)
- generic parameter `V` は trait 側 ── method は generic でないので、 固定 `V` で **object-safe**。 `Box<dyn Node<V>>` が作れる (ADR-0005 D2 の要請を満たす)

### D3 — `Graph<V>` 型

`Graph<V>` は node 集合 + edge (routing table) + boundary を持つ:

- **nodes**: `Box<dyn Node<V>>` の集合 (id 付き)
- **edges**: routing table ── `(source node id, Voyage 分岐)` → target node id
- **boundary**: entry node id / exit node id ── `Graph` が `Node` として振る舞う時の input 受け口 / output 出口

そして **`impl Node<V> for Graph<V>`** (ADR-0005 D6) ── `Graph` を `drive` すると、 entry node から評価器が回り、 exit node の出力が `Graph` の `Voyage` になる。 graph が自己入れ子する。

### D4 — `Bracket` → `Node` adapter

static な `Bracket` を graph に載せる wrapper:

```rust
pub struct BracketNode<B>(pub B);

impl<B, V> Node<V> for BracketNode<B>
where
    B: Bracket<Input = V, Done = V, Reborn = V, Failed = V>,
{
    fn drive(&self, input: V) -> Voyage<V, V, V> {
        Voyage::RoundTrip(self.0.exit(self.0.enter(input)))
    }
}
```

- `Bracket` の 5 関連型のうち `Input` / `Done` / `Reborn` / `Failed` が `V` に collapse する Bracket を adapt (`Active` は内部)
- Bracket-backed node は常に `RoundTrip` を emit (`Bracket` は RoundTrip 専用、 ADR-0006 D4)
- 型が合わない Bracket は consumer 側で変換 node を挟む

### D5 — graph 評価戦略: push 型 work-list

graph 全体の駆動は **push 型** ── node を drive → `Voyage` を得る → routing table で行き先 node に値を push → work-list に積む → 繰り返す。

```text
work-list = [(entry, initial)]
while work-list が空でない:
    (node, input) を取り出す
    match node.drive(input):
        RoundTrip(Done(v))   → 下流 node を work-list に積む
        RoundTrip(Reborn(v)) → (同 node, v) を work-list に積む (self-loop)
        RoundTrip(Failed(v)) → error edge / sink へ
        OneWay               → routing table の Spread policy に従う (D6)
```

**終了条件**: 評価器は exit node が terminal (`Done` / `Failed` / `OneWay`) を出した時点で停止し、 その `Voyage` が `Graph` 全体の出力になる (`impl Node for Graph` の戻り値)。 work-list が尽きても exit 未到達なら明示エラー。

push 型を採る理由 ── `Reborn` の self-loop は **cycle** であり、 topological order は cycle を扱えない。 pull 型 (lazy) は graph に副作用的 node があると semantics が複雑。 push 型は ADR-0003/0004 の 「drive」 mental model の素直な拡張。

### D6 — `Spread` 型は graph 側の routing policy

```rust
pub enum Spread {
    /// fan-out ── 全近傍 node へ flood
    Ok,
    /// terminal ── その node で消費、 edge 無し
    Ng,
}
```

`Voyage::OneWay` は core で bare (ADR-0006 D3)。 `Spread` は **`Voyage` の payload ではなく graph 側の routing 設定**。 routing table が node の OneWay 出力に対し `Spread` policy を持ち、 評価器がそれを引く。

`OneWay(Spread::Ok)` は ADR-0005 D5 の 「空間の再帰」 (fan-out edge)。

### D7 — crate scaffold

```
crates/nostos-graph/
├── Cargo.toml      # package = club-nostos-graph、 lib = nostos_graph
└── src/lib.rs      # Node / Graph / BracketNode / Spread + 評価器
```

- workspace member として追加 (workspace は 2 crate 構成に)
- `club-nostos` (core) に依存。 heterogeneous node の boxing で `alloc` を要する ── `alloc` 依存 (or `std`)
- 命名は CONVENTIONS.md 準拠

## Consequences

### Positive

- generic `<V>` で downcast 無し ── 型安全、 `Any` を avoid、 no_std 寄りを保てる
- `Node<V>` が固定 `V` で object-safe ── ADR-0005 D2 の dyn 要請を満たす
- `drive` が `Voyage` を返すことで ADR-0005 OQ6 (OneWay-emitting node) が自然に解決
- push 型評価器が `Reborn` cycle を素直に扱える

### Negative

- `Graph<V>` homogeneous ── port ごとに型が違う graph は組めない (heterogeneous は後続)
- `BracketNode` の `Input=Done=Reborn=Failed=V` 制約は強い ── 多くの実 Bracket は変換 node を要する

### Neutral

- 評価器の並行実行・非同期は本 ADR scope 外 ── push 型 work-list は将来 work-stealing 等へ拡張余地

## Resolved Questions

review (2026-05-18) で確定:

1. **`Node::drive` の self** → **`&self`** (D2)。 `Bracket` と一貫。 stateful node は interior mutability (`Cell` / `RefCell`) で対応
2. **boundary の表現** → **single entry / single exit** (D3)。 `Graph: Node` (単一 input → 単一 `Voyage`) の必然的帰結
3. **評価の終了条件** → **exit node の terminal 到達** (D5)。 exit node が terminal を出したら graph 全体の `Voyage`。 work-list が尽きて exit 未到達なら明示エラー

## Open Questions (後続)

1. **heterogeneous value** ── `Graph<V>` homogeneous で start。 port ごとに型が違う graph (visual editor で型付き port を繋ぐ) を後続で対応するか、 その時 `V` を sum 型にして consumer に委ねるか
2. **graph 構築 API** ── builder pattern か直接 mutation か ── 実装段階で decide

---

> **昇格** (2026-05-18): review で `Node::drive` の `&self` / single boundary / exit-terminal 終了条件を確定。 本 ADR を `accepted` に昇格。 次は `crates/nostos-graph/` の実装。
