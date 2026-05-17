//! `Driver` trait ── [`Bracket`] を駆動する主体。
//!
//! 詳細設計は [ADR-0004](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0004-bracket-and-driver.md)。

use crate::{Bracket, Outcome};

/// [`Bracket`] を駆動する主体。
///
/// `Driver` は `Reborn` のたび [`next`](Driver::next) に諮り、 次サイクルの入力を
/// 決める。 Human / AI の駆動の違いも、 単発 (lifecycle) ↔ 反復 (loop) の違いも、
/// すべて `next` の実装に還元される ── 単発 Driver は `next` が常に `Err` を返し、
/// `run` はちょうど 1 cycle で終わる。
///
/// # Examples
///
/// ```
/// use nostos::{Bracket, Driver, Outcome};
///
/// struct Countdown;
/// impl Bracket for Countdown {
///     type Input = u32;
///     type Active = u32;
///     type Done = ();
///     type Reborn = u32;
///     type Failed = core::convert::Infallible;
///     fn enter(&self, input: u32) -> u32 {
///         input
///     }
///     fn exit(&self, active: u32) -> Outcome<(), u32, core::convert::Infallible> {
///         if active == 0 {
///             Outcome::Done(())
///         } else {
///             Outcome::Reborn(active - 1)
///         }
///     }
/// }
///
/// // Reborn をそのまま次入力にする反復 Driver。
/// struct Loop;
/// impl Driver<Countdown> for Loop {
///     fn next(&mut self, reborn: u32) -> Result<u32, u32> {
///         Ok(reborn)
///     }
/// }
///
/// let mut driver = Loop;
/// assert_eq!(driver.run(&Countdown, 3), Outcome::Done(()));
/// ```
pub trait Driver<B: Bracket> {
    /// `Reborn` を受けて次サイクルの入力を決める。
    ///
    /// - `Ok(input)` ── 継続。 `input` で次の `enter` を呼ぶ
    /// - `Err(reborn)` ── 打ち切り。 受け取った `Reborn` をそのまま手放す
    fn next(&mut self, reborn: B::Reborn) -> Result<B::Input, B::Reborn>;

    /// `bracket` を `initial` から駆動し、 終端結果を返す。
    ///
    /// `Done` / `Failed` で即終端。 `Reborn` のたび [`next`](Driver::next) に諮り、
    /// `Ok` なら次サイクル、 `Err` ならその `Reborn` を返して終わる。
    fn run(&mut self, bracket: &B, initial: B::Input) -> Outcome<B::Done, B::Reborn, B::Failed> {
        let mut input = initial;
        loop {
            let active = bracket.enter(input);
            match bracket.exit(active) {
                Outcome::Done(o) => return Outcome::Done(o),
                Outcome::Failed(e) => return Outcome::Failed(e),
                Outcome::Reborn(r) => match self.next(r) {
                    Ok(next) => input = next,
                    Err(reborn) => return Outcome::Reborn(reborn),
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用 Bracket ── `0` で `Done`、 それ以外は 1 減らして `Reborn`。
    struct Countdown;

    impl Bracket for Countdown {
        type Input = u32;
        type Active = u32;
        type Done = u32;
        type Reborn = u32;
        type Failed = &'static str;

        fn enter(&self, input: u32) -> u32 {
            input
        }

        fn exit(&self, active: u32) -> Outcome<u32, u32, &'static str> {
            if active == 0 {
                Outcome::Done(0)
            } else {
                Outcome::Reborn(active - 1)
            }
        }
    }

    /// `Reborn` をそのまま次入力にする反復 Driver。
    struct LoopDriver;
    impl Driver<Countdown> for LoopDriver {
        fn next(&mut self, reborn: u32) -> Result<u32, u32> {
            Ok(reborn)
        }
    }

    /// 常に打ち切る単発 Driver。
    struct OnceDriver;
    impl Driver<Countdown> for OnceDriver {
        fn next(&mut self, reborn: u32) -> Result<u32, u32> {
            Err(reborn)
        }
    }

    /// `budget` 回まで継続する Driver。
    struct BoundedDriver {
        budget: usize,
    }
    impl Driver<Countdown> for BoundedDriver {
        fn next(&mut self, reborn: u32) -> Result<u32, u32> {
            if self.budget > 0 {
                self.budget -= 1;
                Ok(reborn)
            } else {
                Err(reborn)
            }
        }
    }

    #[test]
    fn loop_driver_runs_to_done() {
        let mut d = LoopDriver;
        // 3 → 2 → 1 → 0 と Reborn、 0 で Done。
        assert_eq!(d.run(&Countdown, 3), Outcome::Done(0));
    }

    #[test]
    fn once_driver_does_single_cycle() {
        let mut d = OnceDriver;
        // enter(3) → exit → Reborn(2) → next=Err → Reborn(2) を返す。
        assert_eq!(d.run(&Countdown, 3), Outcome::Reborn(2));
    }

    #[test]
    fn once_driver_returns_done_when_first_cycle_done() {
        let mut d = OnceDriver;
        // enter(0) → exit → Done(0)。 next は呼ばれない。
        assert_eq!(d.run(&Countdown, 0), Outcome::Done(0));
    }

    #[test]
    fn bounded_driver_stops_after_budget() {
        let mut d = BoundedDriver { budget: 1 };
        // start 3: cycle1 Reborn(2) → next Ok (budget→0)、 cycle2 Reborn(1) → next Err。
        assert_eq!(d.run(&Countdown, 3), Outcome::Reborn(1));
    }

    #[test]
    fn driver_reaches_failed() {
        struct AlwaysFail;
        impl Bracket for AlwaysFail {
            type Input = ();
            type Active = ();
            type Done = ();
            type Reborn = ();
            type Failed = &'static str;
            fn enter(&self, _input: ()) {}
            fn exit(&self, _active: ()) -> Outcome<(), (), &'static str> {
                Outcome::Failed("nope")
            }
        }
        struct AnyDriver;
        impl Driver<AlwaysFail> for AnyDriver {
            fn next(&mut self, reborn: ()) -> Result<(), ()> {
                Ok(reborn)
            }
        }
        let mut d = AnyDriver;
        assert_eq!(d.run(&AlwaysFail, ()), Outcome::Failed("nope"));
    }
}
