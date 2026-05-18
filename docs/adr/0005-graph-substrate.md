# ADR-0005 — Axis D: graph 同型 substrate

> **Status**: `proposed` (= draft、 review 待ち)
> **Date**: 2026-05-17
> **Deciders**: mito (with claude Opus 4.7 as conversation partner)
> **Refines**: [ADR-0001](0001-bracket-and-outcome.md) Axis D
> **Builds on**: [ADR-0002](0002-outcome-adt.md) / [ADR-0003](0003-lifecycle-loop-dual.md) / [ADR-0004](0004-bracket-and-driver.md) / [ADR-0006](0006-voyage.md)
> **Consumer signals**: creoui `lifecycle-spine.md` §5、 club-kdl 拡張 KDL (`mem_1Cb7yU5TBrwgVxENNurvoT`)

---

## Context

[ADR-0001](0001-bracket-and-outcome.md) Axis D は 「node graph editor との同型 substrate」 を問うた。 暫定傾きは 「founding 段階では深追り不要・別 crate 化が筋」。 `v0.1.0` で core 3 axes (Outcome / loop / Bracket・Driver) が出揃ったので、 Axis D に進む。

Axis D には既に **2 つの consumer signal** がある:

| signal | 内容 |
|--------|------|
| creoui `lifecycle-spine.md` §5 | Editor Mode の 4 方向 layout (TOP/LEFT/CONTENT/RIGHT/BOTTOM) が 「1 個の bracket node の graph topology を physical に体現」 |
| club-kdl 拡張 KDL (`mem_1Cb7yU5T…`) | KDL 制御ノード (`for`/`if`/`bracket`/`lifecycle`) を nostos primitives に mapping する評価器 ── KDL 評価器 ≈ graph 評価器 |

両者とも **nostos primitive を宣言的に組む front-end**。 Axis D はその 2 つが共通で乗る **graph substrate** を設計する。

## Decision

### D1 — graph model: node = Bracket、 edge = Outcome routing

graph の node は [`Bracket`](0004-bracket-and-driver.md)。 node を駆動すると [`Outcome`](0002-outcome-adt.md) が出て、 その 3 variant が edge routing を決める:

| Outcome | graph 上の routing |
|---------|-------------------|
| `Done(O)` | 下流 node の input へ流れる |
| `Reborn(I)` | self-loop ── 同 node を再 enter (ADR-0003 の loop) |
| `Failed(E)` | error edge / sink へ |

これは ADR-0003 の dual view を graph 上に展開したもの ── 1 node 内の `Reborn` loop と、 node 間の `Done` flow が同じ `Outcome` 値で表現される。 graph 全体が 「Bracket を node、 Outcome を edge」 の有向グラフ。

### D2 — dyn-dispatch bridge: type-erased `Node`

`Bracket` は関連型 5 つを持つため **object-safe でない** ── `Box<dyn Bracket>` は作れない。 一方 graph editor は heterogeneous な node を runtime composition するため **dyn dispatch が必須**。

橋渡し: nostos-graph 側に **type-erased な `Node` trait** を置く。 graph 上を流れる値を単一の graph value 型に unify し、 static な `Bracket` impl を `Node` に adapt する wrapper を提供する。

```text
core:        Bracket (static dispatch、 関連型 5、 v0.1.0 のまま不変)
                │  adapt
graph crate: Node   (object-safe、 graph value 型で type-erased)
                │  Box<dyn Node>
             graph  (heterogeneous node の集合 + edge)
```

`nostos-core` の `Bracket` は static dispatch のまま **変更しない** ── type erasure は graph crate の責務。

### D3 — 別 crate `crates/nostos-graph/`

ADR-0001 暫定通り別 crate とする。

- `nostos-core` は `no_std` / dependency-free を保つ (v0.1.0 の性質を維持)
- `nostos-graph` は heterogeneous node の boxing で `alloc` を要する ── `alloc` or `std` 依存
- workspace member として `crates/nostos-graph/` を追加
- 命名 (CONVENTIONS.md 準拠): package `club-nostos-graph` / lib `nostos_graph`

### D4 — front-end は graph crate の外

creoui の visual editor も club-kdl の KDL evaluator も **graph を構築する front-end** であり、 nostos-graph 自体ではない。

```text
front-end          nostos-graph        nostos-core
─────────          ────────────        ───────────
creoui visual editor ─┐
                      ├─→ graph (Node 群 + edge) ─→ 各 Node が Bracket を駆動
club-kdl KDL evaluator ┘
```

nostos-graph は **semantic substrate のみ**提供する ── visual UI も KDL parser も持たない。 front-end は各々が graph を組み立てて nostos-graph に渡す。

### D5 — Voyage 昇華 (ADR-0006) への追従: OneWay の拡散 routing

[ADR-0006](0006-voyage.md) が core を `Voyage = OneWay | RoundTrip(Outcome)` に昇華した。 graph の routing は `Outcome` から `Voyage` に一般化される ── D1 の routing table が OneWay arm を得る:

| Voyage 値 | graph routing | 再帰軸 |
|-----------|--------------|--------|
| `RoundTrip(Done(O))` | 下流 node へ (payload 付き有向継続) | ── |
| `RoundTrip(Reborn(I))` | self-loop (同 node 再 enter) | **時間** |
| `RoundTrip(Failed(E))` | error edge / sink | ── |
| `OneWay(Spread::Ng)` | edge 無し ── node で消費 (terminal) | ── |
| `OneWay(Spread::Ok)` | fan-out edge ── 全近傍へ flood (payload 無し) | **空間** |

`Reborn` が *時間* の再帰 (同じ道を次の周回) なら、 `OneWay(Spread::Ok)` は *空間* の再帰 (同じ message を次のホップ) ── graph 上で前者は self-loop edge、 後者は fan-out edge として現れる。 graph が再帰を両軸で持つ。

**`Spread` 型は `nostos-graph` の責務** (ADR-0006 D3)。 拡散 (propagation) は node 構造を前提にする topology の概念であり、 単一の旅を扱う `nostos-core` には置かない。 `nostos-graph` が OneWay の edge routing policy として `Spread { Ok, Ng }` を定義する。

> OneWay を *emit* する node の model ── `Bracket` は `Outcome` のみを返す (ADR-0006 D4) ため、 OneWay は Bracket node の exit からは出ない ── は本 ADR では framing に留める。 OneWay-emitting node 種別の具体は Open Questions 参照。

## Consequences

### Positive

- ADR-0003 の 「loop を `Outcome` の値で表現」 が graph に直結 ── `Outcome` がそのまま edge routing になり、 graph が特別な制御構造を持たずに済む
- core を一切変えずに graph 層を足せる ── `Bracket` (static) と `Node` (erased) の 2 層分離
- creoui / club-kdl の 2 front-end が同じ substrate を共有 ── ecosystem の重複を防ぐ

### Negative

- type erasure で `Bracket` の型安全性 (関連型による静的保証) が graph 層では緩む ── graph value 型の設計次第で実行時エラーの余地
- 別 crate で workspace が 2 crate 構成になる ── 管理コスト微増

### Neutral

- Axis E (CGP integration) は本 ADR では扱わない ── graph と CGP は別軸

## Open Questions

1. **graph value 型** ── node 間を流れる値を `Box<dyn Any>` で持つか、 nostos-graph 定義の専用 `Value` enum にするか。 柔軟性 (Any) vs 型の明示性・no_std 寄り (enum)
2. **本 ADR の scope** ── ADR-0005 で `Node` trait の具体 signature・graph データ構造まで decide するか、 framing に留め具体は後続 ADR で深掘るか (Axis D は遠い領域、 ADR-0001 が framing ADR だった先例)
3. **graph 評価戦略** ── graph 全体の駆動を push 型 / pull 型 / topological order のどれにするか。 本 ADR で扱うか後続か
4. **crate 名** ── package `club-nostos-graph` で確定か
5. **node の grouping** (D5 関連、 未考慮) ── routing は現状 1 値 → 1 経路 (or fan-out) の単線。 複数 node を 1 単位に束ね、 **group 境界で routing する** 概念は未着手。 group 内 routing と group 間 routing の階層、 group への一括 fan-out、 group を 1 個の合成 Bracket とみなす圧縮 (= sub-graph の bracket 化) 等。 後続 ADR で扱う
6. **OneWay-emitting node の model** (D5 関連) ── `Bracket` node の exit は `Outcome` のみ (ADR-0006 D4)。 OneWay を graph に注入する node 種別 ── 専用の emitter node か、 `Node` trait 側で `Voyage` を返せるようにするか ── は後続で decide

---

> 本 ADR は draft 段階。 Open Questions を review で詰めてから `accepted` に昇格する。
