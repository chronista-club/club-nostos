English | [日本語](README.ja.md)

# nostos

Bracket, lifecycle, and Outcome primitives for Rust. `#![no_std]`, zero dependencies.

[![Crates.io](https://img.shields.io/crates/v/club-nostos.svg)](https://crates.io/crates/club-nostos)
[![CI](https://github.com/chronista-club/club-nostos/workflows/CI/badge.svg)](https://github.com/chronista-club/club-nostos/actions)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

```toml
[dependencies]
# crates.io package = `club-nostos`, Rust crate identifier = `nostos`
club-nostos = "0.1"
```

```rust
use nostos::{Outcome, drive, drive_bounded, Bracket, Driver, Voyage};
```

A `Result` has two answers: it worked, or it didn't. A surprising amount of real code has a *third*: **not done, but changed — try again from here.** A retry, a loop iteration, a paused job waiting to resume. nostos gives that third answer a type, and builds a small set of primitives around it.

The name is Greek **νόστος** — the *homecoming, transformed*. You return, but not to the same place, and not as the same person.

---

## The three-phase return — `Outcome`

`Outcome` is the core. Every journey ends in one of three arms:

```rust
use nostos::Outcome;

// Outcome<O, I, E>  ──  O: result · I: next input · E: failure
let done:   Outcome<i32, i32, &str> = Outcome::Done(42);    // finished — brought a result home
let reborn: Outcome<i32, i32, &str> = Outcome::Reborn(7);   // transformed — carries the next input
let failed: Outcome<i32, i32, &str> = Outcome::Failed("e"); // gave up
```

`Reborn(I)` is the arm `Result` doesn't have. It isn't success and it isn't an error — it's *"keep going, from this new state."* That single distinction is what the rest of the library is built to exploit.

- `Cycle<T, E> = Outcome<T, T, E>` — the common case where the result and the next input are the same type.
- `From<Result>` maps `Ok → Done` and `Err → Failed` (there's no `Reborn` to invent, so it never produces one).
- `map_done` / `map_reborn` / `map_failed` transform one arm and leave the others untouched.

---

## Driving a loop — `drive` / `drive_bounded`

Run a step as long as it keeps returning `Reborn`:

```rust
use nostos::{drive, Outcome};

// 0 → 1 → 2 reborn, then Done(3).
let result: Result<i32, ()> = drive(0, |x| {
    if x < 3 { Outcome::Reborn(x + 1) } else { Outcome::Done(x) }
});
assert_eq!(result, Ok(3));
```

`drive` only ends on `Done` or `Failed`, so it converges to two arms and returns a plain `Result`.

When you need a ceiling, `drive_bounded` returns an `Outcome` instead — and if it hits the limit, you get the last `Reborn(i)` back. That value *is* the resume point: call again with `i` to continue.

```rust
use nostos::{drive_bounded, Outcome};

// Stop after 2 steps — the last Reborn(2) comes back, meaning "paused, resume from 2".
let out: Outcome<i32, i32, ()> = drive_bounded(0, 2, |x| Outcome::Reborn(x + 1));
assert_eq!(out, Outcome::Reborn(2));
```

---

## Lifecycle as a type — `Bracket` / `Driver`

A `Bracket` is a lifecycle: `enter` opens it, `exit` folds it down into an `Outcome`. A `Driver` decides what happens on each `Reborn` — feed it back in, or stop. **Single-shot versus looping is just a different `Driver`**, not different code.

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

// A looping driver: feed every Reborn straight back as the next input.
struct Loop;
impl Driver<Countdown> for Loop {
    fn next(&mut self, reborn: u32) -> Result<u32, u32> { Ok(reborn) }
}

assert_eq!(Loop.run(&Countdown, 3), Outcome::Done(()));
```

Swap `Loop` for a driver whose `next` returns `Err` and you get a single-shot lifecycle — same `Bracket`, different rhythm. Human-paced, AI-paced, or budget-limited driving all live in `next`.

---

## The outbound leg too — `Voyage`

`Outcome` describes the *return*. `Voyage` adds the one-way trip — something sent with no answer expected (a notification, a broadcast) — while wrapping a whole `Outcome` for the round trip:

```rust
use nostos::{Outcome, Voyage};

let fire: Voyage<&str, (), ()> = Voyage::OneWay("ping");    // sent, no return
let back: Voyage<i32, i32, ()> = Outcome::Done(42).into(); // round trip wraps an Outcome
assert_eq!(fire.one_way(), Some("ping"));
assert_eq!(back.round_trip(), Some(Outcome::Done(42)));
```

It doesn't add a fourth arm to `Outcome`; it *contains* `Outcome` in its `RoundTrip` arm and sits one level above it.

---

## A real example — retrying a git worktree

The first production user is [Vantage Point](https://github.com/chronista-club), which adds git worktrees with retries. The point is that a lock contention **isn't a failure** — it's a `Reborn`: wait and try the same thing again. Only a genuine conflict is `Failed`; success is `Done`.

```rust
use nostos::{drive_bounded, Outcome};

// lock → Reborn (try again) · conflict → Failed (stop) · ok → Done(path)
let outcome = drive_bounded(0, 4, |attempt| match worktree_add() {
    Ok(path)              => Outcome::Done(path),
    Err(e) if e.is_lock() => Outcome::Reborn(attempt + 1),
    Err(e)                => Outcome::Failed(e),
});
```

The three arms become the vocabulary of the operation: *retry*, *give up*, *got it*. With `Result`'s single `Err`, the first two would have collapsed into one.

---

## Workspace crates

| Crate | Description |
|-------|-------------|
| [`nostos-core`](https://github.com/chronista-club/club-nostos/tree/main/crates/nostos-core) | Core primitives. Published on crates.io as `club-nostos`; Rust identifier `nostos`. `Outcome`, `drive` / `drive_bounded`, `Bracket` / `Driver`, `Voyage`. `#![no_std]`, no dependencies. |
| [`nostos-graph`](https://github.com/chronista-club/club-nostos/tree/main/crates/nostos-graph) | Graph substrate. Published as `club-nostos-graph`; identifier `nostos_graph`. A directed graph with `Bracket` nodes and `Voyage` edges — `Node` / `Graph` / `Spread` fan-out. |

> **Naming.** Follows the chronista-club `club-` prefix convention: the prefix is on the published identifier (`club-nostos` on crates.io / GitHub), the lib name is the bare `nostos`. So you depend on `club-nostos` but write `use nostos::…`.

---

## Development

```bash
git clone https://github.com/chronista-club/club-nostos
cd club-nostos
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

## Documentation

The design rationale lives in [`docs/adr/`](docs/adr/) — one Architecture Decision Record per primitive, if you want the *why* behind the types rather than just the *how*.

## License

MIT License — [LICENSE](LICENSE)

---

> **nostos** — Greek **νόστος**, *"the homecoming, transformed."* The hero comes back, but the place has changed and so has he. `Outcome::Reborn` is exactly that shape: you returned, but not unchanged. The name is also of a piece with the Mahāyāna *ekō* of the returning aspect (還相回向) and the tenth ox-herding picture, *entering the marketplace with helping hands* (入鄽垂手) — the journey out and the journey home, made into types.
</content>
