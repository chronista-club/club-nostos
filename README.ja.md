[English](README.md) | 日本語

# nostos

Rust のための bracket / lifecycle / Outcome primitive。 `#![no_std]`、 依存ゼロ。

[![Crates.io](https://img.shields.io/crates/v/club-nostos.svg)](https://crates.io/crates/club-nostos)
[![CI](https://github.com/chronista-club/club-nostos/workflows/CI/badge.svg)](https://github.com/chronista-club/club-nostos/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

```toml
[dependencies]
# crates.io package = `club-nostos`、Rust crate identifier = `nostos`
club-nostos = "0.1"
```

```rust
use nostos::{Outcome, drive, drive_bounded, Bracket, Driver, Voyage};
```

`Result` の答えは 2 つ ── 成功したか、しなかったか。 けれど現実のコードには、しばしば **第三の答え** があります ── **「終わってはいないが、変わった。ここから、もう一度」**。 リトライ、ループの 1 反復、再開を待つ中断ジョブ。 nostos はこの第三の答えに型を与え、その周りに小さな primitive を組み上げます。

名前はギリシャ語 **νόστος** ── *変容を経た帰還*。 戻ってくるが、同じ場所へではなく、同じ自分としてでもない。

---

## 帰還の三相 ── `Outcome`

`Outcome` が核。 すべての旅は 3 つの相のいずれかで終わります。

```rust
use nostos::Outcome;

// Outcome<O, I, E>  ──  O: 成果 · I: 次回入力 · E: 失敗
let done:   Outcome<i32, i32, &str> = Outcome::Done(42);    // 完了 ── 成果を持ち帰った
let reborn: Outcome<i32, i32, &str> = Outcome::Reborn(7);   // 変容 ── 次回入力を伴って次へ
let failed: Outcome<i32, i32, &str> = Outcome::Failed("e"); // 断念
```

`Reborn(I)` が `Result` に無い相です。 成功でもエラーでもなく、 *「この新しい状態から、続ける」*。 この一点の区別こそ、ライブラリ全体が活かそうとしているものです。

- `Cycle<T, E> = Outcome<T, T, E>` ── 成果と次回入力が同型になる頻出パターンのエイリアス。
- `From<Result>` は `Ok → Done` / `Err → Failed`（`Reborn` は作りようがないので生まれない）。
- `map_done` / `map_reborn` / `map_failed` は該当する相だけ変換し、他はそのまま通す。

---

## loop を駆動する ── `drive` / `drive_bounded`

`Reborn` が続く限り step を回します。

```rust
use nostos::{drive, Outcome};

// 0 → 1 → 2 と Reborn、 3 で Done。
let result: Result<i32, ()> = drive(0, |x| {
    if x < 3 { Outcome::Reborn(x + 1) } else { Outcome::Done(x) }
});
assert_eq!(result, Ok(3));
```

`drive` は `Done` か `Failed` でしか終わらない ── 必ず二相に収束するので、戻り値はただの `Result` です。

上限が要るときは `drive_bounded`。 戻り値は `Outcome` のままで、上限に達すると最後の `Reborn(i)` が返ります。 この値がそのまま **再開地点** ── 同じ `i` でまた呼べば続行できます。

```rust
use nostos::{drive_bounded, Outcome};

// 2 step で打ち切り ── 最後の Reborn(2) が返る = 「中断、 2 から再開可能」。
let out: Outcome<i32, i32, ()> = drive_bounded(0, 2, |x| Outcome::Reborn(x + 1));
assert_eq!(out, Outcome::Reborn(2));
```

---

## lifecycle を型にする ── `Bracket` / `Driver`

`Bracket` は lifecycle そのもの ── `enter` で開き、 `exit` で `Outcome` へ畳む。 `Driver` は `Reborn` が出るたびに何をするかを決めます ── 再投入するか、打ち切るか。 **単発か反復かは、ただ `Driver` を差し替えるだけ** で、コードを書き分けません。

```rust
use nostos::{Bracket, Driver, Outcome};

struct Countdown;
impl Bracket for Countdown {
    type Input = u32;
    type Active = u32;
    type Done = ();
    type Reborn = u32;
    type Failed = core::convert::Infallible;

    fn enter(&self, input: u32) -> u32 { input }
    fn exit(&self, active: u32) -> Outcome<(), u32, core::convert::Infallible> {
        if active == 0 { Outcome::Done(()) } else { Outcome::Reborn(active - 1) }
    }
}

// 反復 Driver: Reborn をそのまま次の入力に戻す。
struct Loop;
impl Driver<Countdown> for Loop {
    fn next(&mut self, reborn: u32) -> Result<u32, u32> { Ok(reborn) }
}

assert_eq!(Loop.run(&Countdown, 3), Outcome::Done(()));
```

`next` が `Err` を返す Driver に差し替えれば単発 lifecycle になります ── 同じ `Bracket`、違うリズム。 Human 駆動・AI 駆動・予算つき駆動は、すべて `next` の中に宿ります。

---

## 往相も型にする ── `Voyage`

`Outcome` が表すのは *還り*。 `Voyage` はそこに **一方向の旅** ── 還を待たずに発するもの（notification / broadcast）── を足し、往復のときは `Outcome` をまるごと内包します。

```rust
use nostos::{Outcome, Voyage};

let fire: Voyage<&str, (), ()> = Voyage::OneWay("ping");    // 発して還を待たない
let back: Voyage<i32, i32, ()> = Outcome::Done(42).into(); // 往復は Outcome を内包する
assert_eq!(fire.one_way(), Some("ping"));
assert_eq!(back.round_trip(), Some(Outcome::Done(42)));
```

`Outcome` に 4 つ目の相を足すのではなく、 `RoundTrip` arm に `Outcome` を *内包* した一段上の型です。

---

## 実例 ── git worktree のリトライ

最初の本番 consumer は [Vantage Point](https://github.com/chronista-club) ── git worktree をリトライ付きで追加します。 要点は、 lock 競合は **失敗ではない** ということ。 それは `Reborn` ── 待って同じ操作にもう一度戻る。 真の conflict だけが `Failed`、 成功が `Done` です。

```rust
use nostos::{drive_bounded, Outcome};

// lock → Reborn(再試行) · conflict → Failed(中断) · 成功 → Done(path)
let outcome = drive_bounded(0, 4, |attempt| match worktree_add() {
    Ok(path)              => Outcome::Done(path),
    Err(e) if e.is_lock() => Outcome::Reborn(attempt + 1),
    Err(e)                => Outcome::Failed(e),
});
```

三相がそのまま操作の語彙になります ── *再試行*・*断念*・*取得*。 `Result` の `Err` 一相では、最初の 2 つが 1 つに潰れていました。

---

## ワークスペースクレート

| クレート | 説明 |
|---------|------|
| [`nostos-core`](https://github.com/chronista-club/club-nostos/tree/main/crates/nostos-core) | コア primitive。 crates.io では `club-nostos` として公開、Rust identifier は `nostos`。 `Outcome`、`drive` / `drive_bounded`、`Bracket` / `Driver`、`Voyage`。 `#![no_std]`・依存ゼロ。 |
| [`nostos-graph`](https://github.com/chronista-club/club-nostos/tree/main/crates/nostos-graph) | graph substrate。 `club-nostos-graph` として公開、identifier は `nostos_graph`。 `Bracket` を node、 `Voyage` を edge とする有向グラフ ── `Node` / `Graph` / `Spread` fan-out。 |

> **命名。** chronista-club の `club-` prefix 規約に従います。 prefix は公開識別子（crates.io / GitHub の `club-nostos`）に付き、 lib 名は bare `nostos`。 つまり依存には `club-nostos` と書き、 import は `use nostos::…` です。

---

## 開発

```bash
git clone https://github.com/chronista-club/club-nostos
cd club-nostos
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## ドキュメント

設計の理由は [`docs/adr/`](docs/adr/) に ── primitive ごとに 1 本の Architecture Decision Record。 型の *how* ではなく *why* を知りたいときに。

## ライセンス

MIT License — [LICENSE](LICENSE)

---

> **nostos** ── ギリシャ語 **νόστος**、 *「変容を経た帰還」*。 英雄は還ってくるが、場所は変わり、彼自身も変わっている。 `Outcome::Reborn` はまさにその形 ── 還ったが、元のままではない。 この名は大乗仏教の **還相回向** や、十牛図の第十 **入鄽垂手** とも地続きです ── 往く旅と還る旅を、型にしたもの。
</content>
