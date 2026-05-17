# club-nostos

> *"Climb the ladder of abstraction. nostos ensures you return — to the same place, with new eyes."*

**nostos** ── ギリシャ語 **νόστος**、 「変容を経た帰還」。 『オデュッセイア』 中核概念を、 Rust の bracket / lifecycle / Outcome primitive として実装するライブラリです。

## Status

`v0.1.0` — **core primitives 実装済み**。 Outcome ADT・loop driver・`Bracket` / `Driver` trait が使えます。

```rust
use nostos::{Outcome, Cycle, drive, drive_bounded, Bracket, Driver};
```

設計の議事録は [`docs/adr/`](docs/adr/) ── ADR-0001 が 5 axes を framing し、 ADR-0002〜0004 が core 3 axes を decide。

## Scope

ADR-0001 が 5 つの設計 axis を framing しました。 現状:

- ✅ **Outcome ADT** ── 帰還の三相: `Done(O)` / `Reborn(I)` / `Failed(E)` ([ADR-0002](docs/adr/0002-outcome-adt.md))
- ✅ **Lifecycle ↔ loop dual view** ── 単発実行と反復実行を同一 substrate 上で ([ADR-0003](docs/adr/0003-lifecycle-loop-dual.md))
- ✅ **Bracket primitive** ── 状態遷移の `enter` / Active / `exit` を trait として表現 ([ADR-0004](docs/adr/0004-bracket-and-driver.md))
- ⬜ **Node graph editor との同型 substrate** ── visual programming の back-end (後続・別 crate 想定)
- ⬜ **CGP-style component composition** ── 実行戦略の遅延 inject (後続・別 crate 想定)

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
├── Cargo.toml              # workspace root
├── crates/
│   └── nostos-core/        # 公開 crate: club-nostos (lib = nostos)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── outcome.rs  # Outcome ADT (ADR-0002)
│           ├── drive.rs    # loop driver (ADR-0003)
│           ├── bracket.rs  # Bracket trait (ADR-0004)
│           └── driver.rs   # Driver trait (ADR-0004)
├── docs/
│   └── adr/                # Architecture Decision Records (0001〜0004)
└── .github/workflows/      # CI (fmt / clippy / test)
```

## ライセンス

[MIT](LICENSE) © 2026 Chronista Club
