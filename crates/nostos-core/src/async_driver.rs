//! `AsyncDriver` trait ── [`AsyncBracket`] を駆動する主体 (async)。
//!
//! 詳細設計は [ADR-0011](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0011-async-bracket-and-drive.md)。

use crate::{AsyncBracket, Outcome};

/// [`AsyncBracket`] を駆動する主体 (async)。 sync [`Driver`](crate::Driver) の async 対。
///
/// `Reborn` のたび [`next`](AsyncDriver::next) に諮り、 次サイクルの入力を決める。
/// sync [`Driver`](crate::Driver) と同じく、 単発 (lifecycle) ↔ 反復 (loop) の違いは
/// すべて `next` の実装に還元される ── 単発 Driver は `next` が常に `Err` を返す。
///
/// `next` / `run` が `async fn` (AFIT) である点だけが sync 版と異なる。 戻り future に
/// `Send` 境界は課さない (ADR-0011 D7)。
///
/// # Examples
///
/// ```
/// use nostos::{AsyncBracket, AsyncDriver, Outcome};
///
/// struct Countdown;
/// impl AsyncBracket for Countdown {
///     type Input = u32;
///     type Active = u32;
///     type Done = ();
///     type Reborn = u32;
///     type Failed = core::convert::Infallible;
///     async fn enter(&self, input: u32) -> u32 {
///         input
///     }
///     async fn exit(&self, active: u32) -> Outcome<(), u32, core::convert::Infallible> {
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
/// impl AsyncDriver<Countdown> for Loop {
///     async fn next(&mut self, reborn: u32) -> Result<u32, u32> {
///         Ok(reborn)
///     }
/// }
///
/// # async fn _demo() {
/// let mut driver = Loop;
/// assert_eq!(driver.run(&Countdown, 3).await, Outcome::Done(()));
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait AsyncDriver<B: AsyncBracket> {
    /// `Reborn` を受けて次サイクルの入力を決める (async)。
    ///
    /// - `Ok(input)` ── 継続。 `input` で次の `enter` を呼ぶ
    /// - `Err(reborn)` ── 打ち切り。 受け取った `Reborn` をそのまま手放す
    async fn next(&mut self, reborn: B::Reborn) -> Result<B::Input, B::Reborn>;

    /// `bracket` を `initial` から駆動し、 終端結果を返す (async)。
    ///
    /// `Done` / `Failed` で即終端。 `Reborn` のたび [`next`](AsyncDriver::next) に諮り、
    /// `Ok` なら次サイクル、 `Err` ならその `Reborn` を返して終わる。
    async fn run(
        &mut self,
        bracket: &B,
        initial: B::Input,
    ) -> Outcome<B::Done, B::Reborn, B::Failed> {
        let mut input = initial;
        loop {
            let active = bracket.enter(input).await;
            match bracket.exit(active).await {
                Outcome::Done(o) => return Outcome::Done(o),
                Outcome::Failed(e) => return Outcome::Failed(e),
                Outcome::Reborn(r) => match self.next(r).await {
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
    use crate::block_on;

    /// テスト用 AsyncBracket ── `0` で `Done`、 それ以外は 1 減らして `Reborn`。
    struct Countdown;

    impl AsyncBracket for Countdown {
        type Input = u32;
        type Active = u32;
        type Done = u32;
        type Reborn = u32;
        type Failed = &'static str;

        async fn enter(&self, input: u32) -> u32 {
            input
        }

        async fn exit(&self, active: u32) -> Outcome<u32, u32, &'static str> {
            if active == 0 {
                Outcome::Done(0)
            } else {
                Outcome::Reborn(active - 1)
            }
        }
    }

    /// `Reborn` をそのまま次入力にする反復 Driver。
    struct LoopDriver;
    impl AsyncDriver<Countdown> for LoopDriver {
        async fn next(&mut self, reborn: u32) -> Result<u32, u32> {
            Ok(reborn)
        }
    }

    /// 常に打ち切る単発 Driver。
    struct OnceDriver;
    impl AsyncDriver<Countdown> for OnceDriver {
        async fn next(&mut self, reborn: u32) -> Result<u32, u32> {
            Err(reborn)
        }
    }

    #[test]
    fn loop_driver_runs_to_done() {
        let mut d = LoopDriver;
        assert_eq!(block_on(d.run(&Countdown, 3)), Outcome::Done(0));
    }

    #[test]
    fn once_driver_does_single_cycle() {
        let mut d = OnceDriver;
        // enter(3) → exit → Reborn(2) → next=Err → Reborn(2) を返す。
        assert_eq!(block_on(d.run(&Countdown, 3)), Outcome::Reborn(2));
    }

    #[test]
    fn once_driver_returns_done_when_first_cycle_done() {
        let mut d = OnceDriver;
        // enter(0) → exit → Done(0)。 next は呼ばれない。
        assert_eq!(block_on(d.run(&Countdown, 0)), Outcome::Done(0));
    }

    #[test]
    fn run_reaches_failed() {
        // exit が Failed を返したら run は即終端する (sync driver_reaches_failed と対称)。
        struct AlwaysFail;
        impl AsyncBracket for AlwaysFail {
            type Input = ();
            type Active = ();
            type Done = ();
            type Reborn = ();
            type Failed = &'static str;
            async fn enter(&self, _input: ()) {}
            async fn exit(&self, _active: ()) -> Outcome<(), (), &'static str> {
                Outcome::Failed("nope")
            }
        }
        struct AnyDriver;
        impl AsyncDriver<AlwaysFail> for AnyDriver {
            async fn next(&mut self, reborn: ()) -> Result<(), ()> {
                Ok(reborn)
            }
        }
        let mut d = AnyDriver;
        assert_eq!(block_on(d.run(&AlwaysFail, ())), Outcome::Failed("nope"));
    }
}
