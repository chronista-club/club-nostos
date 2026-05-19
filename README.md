# club-nostos

> *"Climb the ladder of abstraction. nostos ensures you return — to the same place, with new eyes."*

**nostos** ── ギリシャ語 **νόστος**、 「変容を経た帰還」。 『オデュッセイア』 中核概念を、 Rust の bracket / lifecycle / Outcome primitive として実装するライブラリです。

## Status

`v0.1.0` — **core primitives + graph substrate 実装済み**。 2-crate 構成 ── `club-nostos` (core) と `club-nostos-graph`。

```rust
use nostos::{Outcome, Cycle, Voyage, drive, drive_bounded, Bracket, Driver};
use nostos_graph::{Graph, Node, Spread};
```

設計の議事録は [`docs/adr/`](docs/adr/) ── ADR-0001 が 5 axes を framing、 ADR-0002〜0008 が Outcome / loop / Bracket・Driver / Voyage / graph substrate を decide。

## Scope

ADR-0001 が 5 つの設計 axis を framing しました。 現状:

- ✅ **Outcome ADT** ── 帰還の三相: `Done(O)` / `Reborn(I)` / `Failed(E)` ([ADR-0002](docs/adr/0002-outcome-adt.md))
- ✅ **Lifecycle ↔ loop dual view** ── 単発実行と反復実行を同一 substrate 上で ([ADR-0003](docs/adr/0003-lifecycle-loop-dual.md))
- ✅ **Bracket primitive** ── 状態遷移の `enter` / Active / `exit` を trait として表現 ([ADR-0004](docs/adr/0004-bracket-and-driver.md))
- ✅ **Node graph editor との同型 substrate** ── `club-nostos-graph` crate ([ADR-0005](docs/adr/0005-graph-substrate.md) / [ADR-0007](docs/adr/0007-graph-design.md))
- ⬜ **CGP-style component composition** ── 実行戦略の遅延 inject (後続・別 crate 想定)

5 axes の外に、 consumer signal (VP messaging) から **Voyage** ── 往相 (`OneWay`) と還相 (`RoundTrip`) の頂点型 ── を獲得 ([ADR-0006](docs/adr/0006-voyage.md) / [ADR-0008](docs/adr/0008-oneway-payload-and-spread.md))。

## Naming

| Layer | Name |
|-------|------|
| Project / atlas (内部呼称) | `nostos` |
| GitHub repo | `chronista-club/club-nostos` |
| Local checkout (Finder discoverability) | `~/repos/club-nostos/` |
| crates.io publication | `club-nostos` |
| Rust crate identifier (`use ...`) | `nostos` |

> chronista-club ecosystem の **`club-` prefix 命名規則** に従います。 prefix は crates.io / GitHub / Finder に listed される識別子 (package・repo・local dir) に付き、 lib 名は bare name (`nostos`) です。 詳細は [chronista-club CONVENTIONS.md](https://github.com/chronista-club/.github/blob/main/CONVENTIONS.md)。

## Concept

> 「梯子を昇って抽象に到達し、 同じ場所に帰ってきて新しい眼で見る」 ── この往復運動を一語で encode するのが nostos です。
>
> `nostalgia` = νόστος (帰還) + ἄλγος (痛み)。 戻ってきても元には戻れない、 という構造が語源に埋め込まれています。 大乗仏教の **還相回向** や十牛図 **入鄽垂手** とも地続きで、 nostos は 「行って帰る」 という根源的な abstraction 運動を、 Rust の型として扱える形に降ろす試みです。

## Crate のレイアウト

```
club-nostos/
├── Cargo.toml              # workspace root (2-crate)
├── crates/
│   ├── nostos-core/        # 公開 crate: club-nostos (lib = nostos)
│   │   └── src/            # outcome / drive / bracket / driver / voyage
│   └── nostos-graph/       # 公開 crate: club-nostos-graph (lib = nostos_graph)
│       └── src/            # node / graph (Axis D — graph 同型 substrate)
├── docs/
│   └── adr/                # Architecture Decision Records (0001〜0008)
└── .github/workflows/      # CI (fmt / clippy / test)
```

## ライセンス

[MIT](LICENSE) © 2026 Chronista Club
