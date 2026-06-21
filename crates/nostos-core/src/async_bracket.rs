//! `AsyncBracket` trait ── async 版の lifecycle primitive (enter / Active / exit)。
//!
//! 詳細設計は [ADR-0011](https://github.com/chronista-club/club-nostos/blob/main/docs/adr/0011-async-bracket-and-drive.md)。

use crate::Outcome;

/// async 版の状態遷移 lifecycle primitive。 sync [`Bracket`](crate::Bracket) の async 対。
///
/// `enter` で Active 相に入り、 `exit` で [`Outcome`] へ収束する ── 両者が `async fn`
/// (AFIT) である点だけが [`Bracket`](crate::Bracket) と異なる。 sync 版と additive に
/// 並置され、 既存 API は一切変えない (ADR-0011 D1/D3)。
///
/// # `Send` 境界について
///
/// 本 trait は戻り future に `Send` 境界を **課さない** (ADR-0011 D7)。 multithread
/// executor (tokio 等) で `spawn` するため future を `Send` にしたい consumer は、
/// 自分の側で `trait_variant` 等を被せる。 concrete 型を inline で drive する限りは
/// compiler が `Send` を推論するため、 多くの場合そのままで足りる。
///
/// # Examples
///
/// ```
/// use nostos::{AsyncBracket, Outcome};
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
/// # async fn _demo() {
/// let countdown = Countdown;
/// assert_eq!(countdown.exit(countdown.enter(0).await).await, Outcome::Done(()));
/// assert_eq!(countdown.exit(countdown.enter(3).await).await, Outcome::Reborn(2));
/// # }
/// ```
#[allow(async_fn_in_trait)]
pub trait AsyncBracket {
    /// `enter` に渡す入力。
    type Input;
    /// `enter` ～ `exit` の間の作業状態 (= Active 相)。
    type Active;
    /// `exit` が `Done` で返す成果。
    type Done;
    /// `exit` が `Reborn` で返す次回入力。
    type Reborn;
    /// `exit` が `Failed` で返す終端理由。
    type Failed;

    /// bracket を開き、 Active 相に入る (async)。
    async fn enter(&self, input: Self::Input) -> Self::Active;

    /// bracket を閉じ、 [`Outcome`] へ収束する (async)。
    async fn exit(&self, active: Self::Active) -> Outcome<Self::Done, Self::Reborn, Self::Failed>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    /// テスト用 AsyncBracket ── floor 以下で `Done`、 超過で `Reborn`、 1000 超で `Failed`。
    struct Countdown {
        floor: u32,
    }

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
            if active > 1_000 {
                Outcome::Failed("overflow")
            } else if active <= self.floor {
                Outcome::Done(active)
            } else {
                Outcome::Reborn(active - 1)
            }
        }
    }

    #[test]
    fn enter_produces_active() {
        let b = Countdown { floor: 0 };
        assert_eq!(block_on(b.enter(5)), 5);
    }

    #[test]
    fn exit_done_at_floor() {
        let b = Countdown { floor: 2 };
        let out = block_on(async { b.exit(b.enter(2).await).await });
        assert_eq!(out, Outcome::Done(2));
    }

    #[test]
    fn exit_reborn_above_floor() {
        let b = Countdown { floor: 0 };
        let out = block_on(async { b.exit(b.enter(5).await).await });
        assert_eq!(out, Outcome::Reborn(4));
    }

    #[test]
    fn exit_failed_on_overflow() {
        let b = Countdown { floor: 0 };
        let out = block_on(async { b.exit(b.enter(2_000).await).await });
        assert_eq!(out, Outcome::Failed("overflow"));
    }
}
