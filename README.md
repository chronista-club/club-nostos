# club-nostos

> *"Climb the ladder of abstraction. nostos ensures you return — to the same place, with new eyes."*

**nostos** ── ギリシャ語 **νόστος**、 「変容を経た帰還」。 『オデュッセイア』 中核概念を、 Rust の bracket / lifecycle / Outcome primitive として実装するライブラリです。

## Status

`v0.1.0` — **core primitives + graph substrate 実装済み**。 2-crate 構成 ── `club-nostos` (core) と `club-nostos-graph`。

```rust
use nostos::{Outcome, Cycle, Voyage, drive, drive_bounded, Bracket, Driver};
use nostos_graph::{Graph, Node, Spread};
```

設計の議事録は [`docs/adr/`](docs/adr/) ── ADR-0001 が 5 axes を framing、 ADR-0002〜0008 が Outcome / loop / Bracket・Driver / Voyage / graph substrate を decide、 ADR-0009 が CGP 統合の framing。

> **crate ↔ tag の出自**: crates.io `club-nostos` 0.1.0 は main `d820225` から cut しています (= `v0.1.0` tag `bf66ca3` より一歩先、 `voyage.rs` を含む)。 差分は additive のため、 公開済み tag は据え置き、 crate の出自はこの記録を正典とします。 consumer は crates.io を消費するため影響はありません。

## Scope

ADR-0001 が 5 つの設計 axis を framing しました。 現状:

- ✅ **Outcome ADT** ── 帰還の三相: `Done(O)` / `Reborn(I)` / `Failed(E)` ([ADR-0002](docs/adr/0002-outcome-adt.md))
- ✅ **Lifecycle ↔ loop dual view** ── 単発実行と反復実行を同一 substrate 上で ([ADR-0003](docs/adr/0003-lifecycle-loop-dual.md))
- ✅ **Bracket primitive** ── 状態遷移の `enter` / Active / `exit` を trait として表現 ([ADR-0004](docs/adr/0004-bracket-and-driver.md))
- ✅ **Node graph editor との同型 substrate** ── `club-nostos-graph` crate ([ADR-0005](docs/adr/0005-graph-substrate.md) / [ADR-0007](docs/adr/0007-graph-design.md))
- ⬜ **CGP-style component composition** ── 実行戦略の遅延 inject (後続・別 crate 想定)

5 axes の外に、 consumer signal (VP messaging) から **Voyage** ── 往相 (`OneWay`) と還相 (`RoundTrip`) の頂点型 ── を獲得 ([ADR-0006](docs/adr/0006-voyage.md) / [ADR-0008](docs/adr/0008-oneway-payload-and-spread.md))。

## 使い方

```toml
[dependencies]
# lib 名は bare `nostos`、 crates.io package は `club-nostos`
nostos = { package = "club-nostos", version = "0.1" }
```

`nostos` は `#![no_std]`。 alloc も std も要求しません。

### `Outcome<O, I, E>` ── 帰還の三相

すべての出発点。 旅の結末を **持ち帰った成果 (`Done`)** / **変容して次の生へ (`Reborn`)** / **失敗 (`Failed`)** の三相で表す ADT です。 `Reborn` が `Result` に無い第三の相で、 「終わっていないが、 同じ場所には戻らない」 という nostos の核を型にしています。

```rust
use nostos::Outcome;

let o: Outcome<i32, i32, &str> = Outcome::Done(42);
assert!(o.is_done());
assert_eq!(o.done(), Some(42));

// Result からの変換は Done / Failed の二相のみ (Reborn は生まれない)
let from_result: Outcome<i32, i32, &str> = Ok(7).into();
assert_eq!(from_result.done(), Some(7));
```

### `drive` / `drive_bounded` ── lifecycle ↔ loop dual

`Reborn` が続く限り step を駆動する loop driver。 単発の lifecycle と反復 loop を同じ substrate で扱う ([ADR-0003](docs/adr/0003-lifecycle-loop-dual.md))。

```rust
use nostos::{drive, Outcome};

// 0 → 1 → 2 と Reborn、 3 で Done。 終端は必ず Done/Failed の二相に収束 → Result。
let result: Result<i32, ()> = drive(0, |x| {
    if x < 3 { Outcome::Reborn(x + 1) } else { Outcome::Done(x) }
});
assert_eq!(result, Ok(3));
```

`drive_bounded(initial, max_steps, step)` は上限付き。 上限に達すると **`Reborn` のまま** 返るため、 戻り値は `Result` ではなく `Outcome` (= 「未収束」 を型で表現できる)。

### `Bracket` + `Driver` ── enter / active / exit を型に

状態遷移の境界 (`enter` で開き、 `exit` で `Outcome` に畳む) を trait 化したのが `Bracket`。 `exit` が `Reborn` を返したとき **次の入力を決めて再投入する戦略** を担うのが `Driver` で、 `Driver::run` が両者を回します ([ADR-0004](docs/adr/0004-bracket-and-driver.md))。

```rust
use nostos::{Bracket, Driver, Outcome};

// Bracket:  enter(Input) -> Active,  exit(Active) -> Outcome<Done, Reborn, Failed>
// Driver:   next(Reborn) -> Result<Input, Reborn>   (= 再投入する / 打ち切る)
//           run(&bracket, initial) -> Outcome<Done, Reborn, Failed>
```

### `Voyage<O, I, E>` ── 往相と還相

`Outcome` を内包する一段上の頂点型。 **`OneWay`** は還を持たない往相 (notification / broadcast)、 **`RoundTrip`** は往って還る還相で `Outcome` 三相を内包します。 4 つ目の variant を足すのではなく、 往復という上位概念で `Outcome` を **昇華** したものです ([ADR-0006](docs/adr/0006-voyage.md))。

```rust
use nostos::{Voyage, Outcome};

let fire: Voyage<i32, i32, ()> = Voyage::OneWay(1);     // 発して還を待たない
let back: Voyage<i32, i32, ()> = Outcome::Done(42).into(); // RoundTrip へ昇華
assert!(fire.is_one_way());
assert_eq!(back.round_trip(), Some(Outcome::Done(42)));
```

### 最初の実利用 ── VP の worktree retry

club-nostos の最初の本番 consumer は [Vantage Point](https://github.com/chronista-club/vantage-point) の lane コマンド (PR #572)。 `git worktree add` をリトライ付きで実行する処理を、 hand-roll した retry loop から `drive_bounded` へ着替えました (振る舞い不変):

```rust
use nostos::{drive_bounded, Outcome};

// lock 競合 → Reborn(次の試行へ) / conflict 等 → Failed(即中断) / 成功 → Done(path)
let outcome = drive_bounded(0, 4, |attempt| match worktree_add() {
    Ok(path)              => Outcome::Done(path),
    Err(e) if e.is_lock() => Outcome::Reborn(attempt + 1),
    Err(e)                => Outcome::Failed(e),
});
```

retry の「待って同じ操作に戻る」 が `Reborn`、 「持ち帰った worktree」 が `Done`、 「conflict で旅を断念」 が `Failed` ── nostos の三相がそのまま lane の語彙になります。

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
│   └── adr/                # Architecture Decision Records (0001〜0009)
└── .github/workflows/      # CI (fmt / clippy / test)
```

## ライセンス

[MIT](LICENSE) © 2026 Chronista Club
